use hickory_proto::op::{MessageType, OpCode, ResponseCode};
use hickory_proto::rr::RecordType;
use hickory_proto::serialize::binary::{BinDecodable, BinEncodable};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::{debug, error, info, warn};

use crate::resolver::KubernetesResolver;

pub struct DnsServer {
    resolver: Arc<KubernetesResolver>,
    addr: SocketAddr,
}

impl DnsServer {
    pub fn new(resolver: Arc<KubernetesResolver>, addr: SocketAddr) -> Self {
        Self { resolver, addr }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let socket = Arc::new(UdpSocket::bind(self.addr).await?);
        info!("DNS server bound to UDP {}", self.addr);

        let mut buf = vec![0u8; 512];

        loop {
            match socket.recv_from(&mut buf).await {
                Ok((len, src)) => {
                    debug!("Received {} bytes from {}", len, src);
                    let data = buf[..len].to_vec();
                    let resolver = self.resolver.clone();
                    let socket = socket.clone();

                    // Spawn a task to handle this request
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_request(data, src, resolver, socket).await {
                            warn!("Error handling DNS request from {}: {}", src, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Error receiving UDP packet: {}", e);
                }
            }
        }
    }

    async fn handle_request(
        data: Vec<u8>,
        src: SocketAddr,
        resolver: Arc<KubernetesResolver>,
        socket: Arc<UdpSocket>,
    ) -> anyhow::Result<()> {
        use hickory_proto::serialize::binary::BinDecoder;

        debug!("Processing {} bytes from {}", data.len(), src);

        // Parse incoming DNS query
        let mut decoder = BinDecoder::new(&data);
        let message = match hickory_proto::op::Message::read(&mut decoder) {
            Ok(msg) => msg,
            Err(e) => {
                warn!("Failed to parse DNS message from {} ({} bytes): {}", src, data.len(), e);
                debug!("Raw data: {:?}", &data[..std::cmp::min(32, data.len())]);
                return Ok(());
            }
        };

        debug!(
            "DNS query from {}: id={}, opcode={:?}, questions={}",
            src,
            message.id(),
            message.op_code(),
            message.query_count()
        );

        // Create response
        let mut response = hickory_proto::op::Message::new();
        response.set_id(message.id());
        response.set_message_type(MessageType::Response);
        response.set_op_code(OpCode::Query);
        response.set_recursion_desired(message.recursion_desired());
        response.set_recursion_available(false);

        // Process queries
        let mut found_answer = false;
        for query in message.queries() {
            debug!(
                "  Query: {} {:?} {:?}",
                query.name(),
                query.query_type(),
                query.query_class()
            );

            response.add_query(query.clone());

            // Only handle A and AAAA queries for now
            match query.query_type() {
                RecordType::A | RecordType::AAAA | RecordType::SRV => {
                    if let Some(records) = resolver.lookup(query.name()) {
                        for record in records {
                            // Filter by query type
                            if record.record_type() == query.query_type() {
                                response.add_answer(record);
                                found_answer = true;
                            }
                        }
                    }
                }
                _ => {
                    debug!("Unsupported query type: {:?}", query.query_type());
                }
            }
        }

        if found_answer {
            response.set_response_code(ResponseCode::NoError);
            debug!("  Response: {} answers", response.answer_count());
        } else {
            response.set_response_code(ResponseCode::NXDomain);
            debug!("  Response: NXDomain (no records found)");
        }

        // Encode and send response
        let response_bytes = match response.to_bytes() {
            Ok(bytes) => bytes,
            Err(e) => {
                warn!("Failed to encode DNS response: {}", e);
                return Ok(());
            }
        };

        // Send response back to client using the same socket
        socket.send_to(&response_bytes, src).await?;

        Ok(())
    }
}
