//! WebSocket streaming support for exec, attach, and port-forward
//!
//! Proxies exec requests to the kubelet's HTTP endpoint,
//! keeping the API server runtime-agnostic.

use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use rusternetes_common::resources::Pod;
use tracing::{debug, error, info};

/// Handle WebSocket exec by proxying to the kubelet
pub async fn handle_ws_exec(
    mut socket: WebSocket,
    pod: Pod,
    container_name: String,
    command: Vec<String>,
    _stdin: bool,
    _stdout: bool,
    _stderr: bool,
    tty: bool,
) {
    let container_id = format!("{}_{}", pod.metadata.name, container_name);

    let node_name = pod
        .spec
        .as_ref()
        .and_then(|s| s.node_name.as_deref())
        .unwrap_or("node-1");

    let (host, port) = match node_name {
        "node-1" => ("rusternetes-kubelet", 10250),
        "node-2" => ("rusternetes-kubelet2", 10251),
        _ => ("rusternetes-kubelet", 10250),
    };

    let command_str = command.join(",");
    let url = format!(
        "http://{}:{}/exec/{}?command={}&tty={}",
        host, port, container_id,
        urlencoding_encode(&command_str),
        tty
    );

    debug!("WS exec proxying to kubelet: {}", url);

    // Collect stdin from WebSocket (first message if any)
    let stdin_data = match tokio::time::timeout(
        std::time::Duration::from_millis(100),
        socket.next(),
    )
    .await
    {
        Ok(Some(Ok(Message::Binary(data)))) => data.to_vec(),
        Ok(Some(Ok(Message::Text(text)))) => text.into_bytes(),
        _ => Vec::new(),
    };

    // Proxy to kubelet
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_default();

    match client.post(&url).body(stdin_data).send().await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(result) => {
                        if let Some(out) = result.get("stdout").and_then(|v| v.as_str()) {
                            if !out.is_empty() {
                                let _ = socket
                                    .send(Message::Binary(
                                        std::iter::once(1u8) // stdout channel prefix
                                            .chain(out.bytes())
                                            .collect(),
                                    ))
                                    .await;
                            }
                        }
                        if let Some(err) = result.get("stderr").and_then(|v| v.as_str()) {
                            if !err.is_empty() {
                                let _ = socket
                                    .send(Message::Binary(
                                        std::iter::once(2u8) // stderr channel prefix
                                            .chain(err.bytes())
                                            .collect(),
                                    ))
                                    .await;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse kubelet exec response: {}", e);
                        let _ = socket
                            .send(Message::Text(format!("Exec failed: {}", e).into()))
                            .await;
                    }
                }
            } else {
                let body = response.text().await.unwrap_or_default();
                let _ = socket
                    .send(Message::Text(format!("Exec failed: {}", body).into()))
                    .await;
            }
        }
        Err(e) => {
            error!("Failed to proxy exec to kubelet: {}", e);
            let _ = socket
                .send(Message::Text(
                    format!("Failed to connect to kubelet: {}", e).into(),
                ))
                .await;
        }
    }

    let _ = socket.close().await;
}

/// Handle WebSocket attach
pub async fn handle_ws_attach(
    mut socket: WebSocket,
    pod: Pod,
    container_name: String,
    _stdin: bool,
    _stdout: bool,
    _stderr: bool,
    _tty: bool,
) {
    info!(
        "WS attach: pod={}, container={}",
        pod.metadata.name, container_name
    );
    let _ = socket
        .send(Message::Text(
            "Attach not fully implemented in proxy mode".into(),
        ))
        .await;
    let _ = socket.close().await;
}

/// Simple URL encoding
fn urlencoding_encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            b' ' => "+".to_string(),
            _ => format!("%{:02X}", b),
        })
        .collect()
}

/// Alias for backward compatibility with pod_subresources.rs
pub async fn handle_exec_websocket(
    socket: WebSocket,
    pod: Pod,
    container_name: String,
    command: Vec<String>,
    stdin: bool,
    stdout: bool,
    stderr: bool,
    tty: bool,
) {
    handle_ws_exec(socket, pod, container_name, command, stdin, stdout, stderr, tty).await
}

/// Alias for backward compatibility
pub async fn handle_attach_websocket(
    socket: WebSocket,
    pod: Pod,
    container_name: String,
    stdin: bool,
    stdout: bool,
    stderr: bool,
    tty: bool,
) {
    handle_ws_attach(socket, pod, container_name, stdin, stdout, stderr, tty).await
}

/// Handle WebSocket port-forward
pub async fn handle_portforward_websocket(
    mut socket: WebSocket,
    pod: Pod,
    ports: Vec<u16>,
) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    let pod_ip = match pod.status.as_ref().and_then(|s| s.pod_ip.as_ref()) {
        Some(ip) => ip.clone(),
        None => {
            let _ = socket.send(Message::Text("Pod has no IP".into())).await;
            let _ = socket.close().await;
            return;
        }
    };

    for port in &ports {
        let target = format!("{}:{}", pod_ip, port);
        match TcpStream::connect(&target).await {
            Ok(tcp) => {
                let (mut tcp_read, mut tcp_write) = tcp.into_split();
                // Simple forward: read from TCP, send to WebSocket
                let mut buf = vec![0u8; 8192];
                loop {
                    match tcp_read.read(&mut buf).await {
                        Ok(0) => break,
                        Ok(n) => {
                            if socket.send(Message::Binary(buf[..n].to_vec().into())).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
            Err(e) => {
                let _ = socket.send(Message::Text(format!("Failed to connect to {}: {}", target, e).into())).await;
            }
        }
    }

    let _ = socket.close().await;
}
