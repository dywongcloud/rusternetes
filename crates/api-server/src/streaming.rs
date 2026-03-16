//! WebSocket streaming support for exec, attach, and port-forward
//!
//! Provides WebSocket-based streaming for interactive pod operations

use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use rusternetes_common::resources::Pod;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::Mutex;
use tracing::{debug, error, warn};

/// Handle WebSocket connection for exec
pub async fn handle_exec_websocket(
    mut socket: WebSocket,
    pod: Pod,
    container_name: String,
    command: Vec<String>,
    stdin: bool,
    stdout: bool,
    stderr: bool,
    tty: bool,
) {
    debug!(
        "WebSocket exec: pod={}, container={}, command={:?}, stdin={}, stdout={}, stderr={}, tty={}",
        pod.metadata.name, container_name, command, stdin, stdout, stderr, tty
    );

    // Get the container runtime command
    // For Podman/Docker, we'll use podman exec / docker exec
    let container_id = format!("{}-{}", pod.metadata.name, container_name);

    // Build the exec command
    let mut cmd = Command::new("podman");
    cmd.arg("exec");

    if tty {
        cmd.arg("-it");
    } else {
        cmd.arg("-i");
    }

    cmd.arg(&container_id);
    cmd.args(&command);

    // Configure stdio
    if stdin {
        cmd.stdin(std::process::Stdio::piped());
    }
    if stdout {
        cmd.stdout(std::process::Stdio::piped());
    }
    if stderr {
        cmd.stderr(std::process::Stdio::piped());
    }

    // Spawn the command
    match cmd.spawn() {
        Ok(mut child) => {
            debug!("Spawned exec command for container {}", container_id);

            // Split socket into sender and receiver
            let (socket_tx, mut socket_rx) = socket.split();
            let socket_tx = Arc::new(Mutex::new(socket_tx));

            // Handle stdin if requested
            if stdin {
                if let Some(mut child_stdin) = child.stdin.take() {
                    tokio::spawn(async move {
                        while let Some(Ok(msg)) = socket_rx.next().await {
                            if let Message::Binary(data) = msg {
                                // Check if this is a multiplexed message
                                if let Some(decoded) = MultiplexedMessage::decode(&data) {
                                    if matches!(decoded.channel, StreamChannel::Stdin) {
                                        use tokio::io::AsyncWriteExt;
                                        if let Err(e) = child_stdin.write_all(&decoded.data).await {
                                            error!("Failed to write to stdin: {}", e);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    });
                }
            }

            // Handle stdout if requested
            if stdout {
                if let Some(mut child_stdout) = child.stdout.take() {
                    let tx = Arc::clone(&socket_tx);
                    tokio::spawn(async move {
                        use tokio::io::AsyncReadExt;
                        let mut buffer = vec![0u8; 8192];
                        loop {
                            match child_stdout.read(&mut buffer).await {
                                Ok(0) => break, // EOF
                                Ok(n) => {
                                    let msg = MultiplexedMessage {
                                        channel: StreamChannel::Stdout,
                                        data: buffer[..n].to_vec(),
                                    };
                                    if let Err(e) =
                                        tx.lock().await.send(Message::Binary(msg.encode())).await
                                    {
                                        error!("Failed to send stdout: {}", e);
                                        break;
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to read stdout: {}", e);
                                    break;
                                }
                            }
                        }
                    });
                }
            }

            // Handle stderr if requested
            if stderr {
                if let Some(mut child_stderr) = child.stderr.take() {
                    let tx = Arc::clone(&socket_tx);
                    tokio::spawn(async move {
                        use tokio::io::AsyncReadExt;
                        let mut buffer = vec![0u8; 8192];
                        loop {
                            match child_stderr.read(&mut buffer).await {
                                Ok(0) => break, // EOF
                                Ok(n) => {
                                    let msg = MultiplexedMessage {
                                        channel: StreamChannel::Stderr,
                                        data: buffer[..n].to_vec(),
                                    };
                                    if let Err(e) =
                                        tx.lock().await.send(Message::Binary(msg.encode())).await
                                    {
                                        error!("Failed to send stderr: {}", e);
                                        break;
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to read stderr: {}", e);
                                    break;
                                }
                            }
                        }
                    });
                }
            }

            // Wait for command to complete
            match child.wait().await {
                Ok(status) => {
                    debug!("Exec command completed with status: {}", status);
                    let _ = socket_tx.lock().await.close().await;
                }
                Err(e) => {
                    error!("Exec command failed: {}", e);
                    let _ = socket_tx.lock().await.close().await;
                }
            }
        }
        Err(e) => {
            error!("Failed to spawn exec command: {}", e);
            let error_msg = MultiplexedMessage {
                channel: StreamChannel::Error,
                data: format!("Failed to execute command: {}", e).into_bytes(),
            };
            let _ = socket.send(Message::Binary(error_msg.encode())).await;
            let _ = socket.close().await;
        }
    }
}

/// Handle WebSocket connection for attach
pub async fn handle_attach_websocket(
    mut socket: WebSocket,
    pod: Pod,
    container_name: String,
    stdin: bool,
    stdout: bool,
    stderr: bool,
    tty: bool,
) {
    debug!(
        "WebSocket attach: pod={}, container={}, stdin={}, stdout={}, stderr={}, tty={}",
        pod.metadata.name, container_name, stdin, stdout, stderr, tty
    );

    let container_id = format!("{}-{}", pod.metadata.name, container_name);

    // Build the attach command
    let mut cmd = Command::new("podman");
    cmd.arg("attach");

    if !tty {
        cmd.arg("--no-stdin");
    }

    cmd.arg(&container_id);

    // Configure stdio
    if stdin {
        cmd.stdin(std::process::Stdio::piped());
    }
    if stdout {
        cmd.stdout(std::process::Stdio::piped());
    }
    if stderr {
        cmd.stderr(std::process::Stdio::piped());
    }

    // Execute attach
    match cmd.spawn() {
        Ok(mut child) => {
            debug!("Attached to container {}", container_id);

            // Split socket into sender and receiver
            let (socket_tx, mut socket_rx) = socket.split();
            let socket_tx = Arc::new(Mutex::new(socket_tx));

            // Handle stdin if requested
            if stdin {
                if let Some(mut child_stdin) = child.stdin.take() {
                    tokio::spawn(async move {
                        while let Some(Ok(msg)) = socket_rx.next().await {
                            if let Message::Binary(data) = msg {
                                if let Some(decoded) = MultiplexedMessage::decode(&data) {
                                    if matches!(decoded.channel, StreamChannel::Stdin) {
                                        use tokio::io::AsyncWriteExt;
                                        if let Err(e) = child_stdin.write_all(&decoded.data).await {
                                            error!("Failed to write to stdin: {}", e);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    });
                }
            }

            // Handle stdout if requested
            if stdout {
                if let Some(mut child_stdout) = child.stdout.take() {
                    let tx = Arc::clone(&socket_tx);
                    tokio::spawn(async move {
                        use tokio::io::AsyncReadExt;
                        let mut buffer = vec![0u8; 8192];
                        loop {
                            match child_stdout.read(&mut buffer).await {
                                Ok(0) => break,
                                Ok(n) => {
                                    let msg = MultiplexedMessage {
                                        channel: StreamChannel::Stdout,
                                        data: buffer[..n].to_vec(),
                                    };
                                    if let Err(e) =
                                        tx.lock().await.send(Message::Binary(msg.encode())).await
                                    {
                                        error!("Failed to send stdout: {}", e);
                                        break;
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to read stdout: {}", e);
                                    break;
                                }
                            }
                        }
                    });
                }
            }

            // Handle stderr if requested
            if stderr {
                if let Some(mut child_stderr) = child.stderr.take() {
                    let tx = Arc::clone(&socket_tx);
                    tokio::spawn(async move {
                        use tokio::io::AsyncReadExt;
                        let mut buffer = vec![0u8; 8192];
                        loop {
                            match child_stderr.read(&mut buffer).await {
                                Ok(0) => break,
                                Ok(n) => {
                                    let msg = MultiplexedMessage {
                                        channel: StreamChannel::Stderr,
                                        data: buffer[..n].to_vec(),
                                    };
                                    if let Err(e) =
                                        tx.lock().await.send(Message::Binary(msg.encode())).await
                                    {
                                        error!("Failed to send stderr: {}", e);
                                        break;
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to read stderr: {}", e);
                                    break;
                                }
                            }
                        }
                    });
                }
            }

            // Wait for process to complete
            match child.wait().await {
                Ok(status) => {
                    debug!("Attach completed with status: {}", status);
                    let _ = socket_tx.lock().await.close().await;
                }
                Err(e) => {
                    error!("Attach failed: {}", e);
                    let _ = socket_tx.lock().await.close().await;
                }
            }
        }
        Err(e) => {
            error!("Failed to attach to container: {}", e);
            let error_msg = MultiplexedMessage {
                channel: StreamChannel::Error,
                data: format!("Failed to attach: {}", e).into_bytes(),
            };
            let _ = socket.send(Message::Binary(error_msg.encode())).await;
            let _ = socket.close().await;
        }
    }
}

/// Handle WebSocket connection for port-forward
pub async fn handle_portforward_websocket(mut socket: WebSocket, pod: Pod, ports: Vec<u16>) {
    debug!(
        "WebSocket port-forward: pod={}, ports={:?}",
        pod.metadata.name, ports
    );

    // For now, send a message explaining port-forward requires special handling
    warn!("Port-forward not yet fully implemented - requires TCP proxy");

    let error_msg = MultiplexedMessage {
        channel: StreamChannel::Error,
        data: b"Port-forward feature requires additional TCP proxy implementation".to_vec(),
    };

    let _ = socket.send(Message::Binary(error_msg.encode())).await;
    let _ = socket.close().await;
}

/// Stream channel for multiplexed I/O
pub enum StreamChannel {
    Stdin,
    Stdout,
    Stderr,
    Error,
    Resize,
}

impl StreamChannel {
    /// Get the channel ID byte
    pub fn id(&self) -> u8 {
        match self {
            StreamChannel::Stdin => 0,
            StreamChannel::Stdout => 1,
            StreamChannel::Stderr => 2,
            StreamChannel::Error => 3,
            StreamChannel::Resize => 4,
        }
    }

    /// Parse channel from byte
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(StreamChannel::Stdin),
            1 => Some(StreamChannel::Stdout),
            2 => Some(StreamChannel::Stderr),
            3 => Some(StreamChannel::Error),
            4 => Some(StreamChannel::Resize),
            _ => None,
        }
    }
}

/// Multiplex message with channel information
pub struct MultiplexedMessage {
    pub channel: StreamChannel,
    pub data: Vec<u8>,
}

impl MultiplexedMessage {
    /// Encode message for WebSocket transmission
    pub fn encode(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(self.data.len() + 1);
        result.push(self.channel.id());
        result.extend_from_slice(&self.data);
        result
    }

    /// Decode message from WebSocket
    pub fn decode(data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }

        let channel = StreamChannel::from_id(data[0])?;
        let message_data = data[1..].to_vec();

        Some(MultiplexedMessage {
            channel,
            data: message_data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_channel_ids() {
        assert_eq!(StreamChannel::Stdin.id(), 0);
        assert_eq!(StreamChannel::Stdout.id(), 1);
        assert_eq!(StreamChannel::Stderr.id(), 2);
        assert_eq!(StreamChannel::Error.id(), 3);
        assert_eq!(StreamChannel::Resize.id(), 4);
    }

    #[test]
    fn test_stream_channel_from_id() {
        assert!(matches!(
            StreamChannel::from_id(0),
            Some(StreamChannel::Stdin)
        ));
        assert!(matches!(
            StreamChannel::from_id(1),
            Some(StreamChannel::Stdout)
        ));
        assert!(matches!(
            StreamChannel::from_id(2),
            Some(StreamChannel::Stderr)
        ));
        assert!(StreamChannel::from_id(255).is_none());
    }

    #[test]
    fn test_multiplexed_message_encode_decode() {
        let msg = MultiplexedMessage {
            channel: StreamChannel::Stdout,
            data: b"Hello, World!".to_vec(),
        };

        let encoded = msg.encode();
        assert_eq!(encoded[0], 1); // Stdout channel

        let decoded = MultiplexedMessage::decode(&encoded).unwrap();
        assert_eq!(decoded.channel.id(), StreamChannel::Stdout.id());
        assert_eq!(decoded.data, b"Hello, World!");
    }

    #[test]
    fn test_multiplexed_message_empty() {
        assert!(MultiplexedMessage::decode(&[]).is_none());
    }
}
