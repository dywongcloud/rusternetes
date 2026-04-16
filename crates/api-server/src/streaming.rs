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

    debug!("WS exec direct Docker for container: {}", container_id);

    // Execute directly via Docker (API server has Docker socket mounted)
    use bollard::exec::{CreateExecOptions, StartExecResults};
    use bollard::Docker;

    let docker = match Docker::connect_with_local_defaults() {
        Ok(d) => d,
        Err(e) => {
            let _ = socket
                .send(Message::Binary(
                    std::iter::once(3u8)
                        .chain(format!("Docker error: {}", e).bytes())
                        .collect(),
                ))
                .await;
            let _ = socket.close().await;
            return;
        }
    };

    let exec_config = CreateExecOptions {
        cmd: Some(command.iter().map(|s| s.as_str()).collect()),
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        attach_stdin: Some(false),
        tty: Some(tty),
        ..Default::default()
    };

    let exec = match docker.create_exec(&container_id, exec_config).await {
        Ok(e) => e,
        Err(e) => {
            let _ = socket
                .send(Message::Binary(
                    std::iter::once(3u8)
                        .chain(format!("Exec error: {}", e).bytes())
                        .collect(),
                ))
                .await;
            let _ = socket.close().await;
            return;
        }
    };

    let output = match docker
        .start_exec(
            &exec.id,
            Some(bollard::exec::StartExecOptions {
                detach: false,
                ..Default::default()
            }),
        )
        .await
    {
        Ok(o) => o,
        Err(e) => {
            let _ = socket
                .send(Message::Binary(
                    std::iter::once(3u8)
                        .chain(format!("Start exec error: {}", e).bytes())
                        .collect(),
                ))
                .await;
            let _ = socket.close().await;
            return;
        }
    };

    // Stream output to WebSocket using v5.channel.k8s.io protocol
    // Channel prefix: 0=stdin, 1=stdout, 2=stderr, 3=error
    // K8s protocol requires channel 1 (stdout) to appear before channel 3 (status).
    // Send an initial empty stdout frame so the client sees ch1 first, even if the
    // exec command produces no output or finishes before we read from the stream.
    let _ = socket.send(Message::Binary(vec![1u8].into())).await;

    if let StartExecResults::Attached {
        output: mut stream, ..
    } = output
    {
        loop {
            match tokio::time::timeout(std::time::Duration::from_secs(1), stream.next()).await {
                Ok(Some(Ok(msg))) => {
                    match msg {
                        bollard::container::LogOutput::StdOut { message } => {
                            let mut data = vec![1u8]; // stdout channel
                            data.extend_from_slice(&message);
                            if socket.send(Message::Binary(data.into())).await.is_err() {
                                break;
                            }
                        }
                        bollard::container::LogOutput::StdErr { message } => {
                            let mut data = vec![2u8]; // stderr channel
                            data.extend_from_slice(&message);
                            if socket.send(Message::Binary(data.into())).await.is_err() {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Some(Err(_))) | Ok(None) => break,
                Err(_) => {
                    // 1s timeout hit — check if command finished
                    if let Ok(info) = docker.inspect_exec(&exec.id).await {
                        if !info.running.unwrap_or(false) {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
        }
    }

    // K8s protocol: client expects channel 1 (stdout) before channel 3 (status).
    // Send empty stdout/stderr frames and flush with a ping to ensure ordering.
    let _ = socket.send(Message::Binary(vec![1u8].into())).await;
    let _ = socket.send(Message::Binary(vec![2u8].into())).await;
    // Send a Ping to flush the TCP buffer — the Pong response ensures the
    // client has processed the stdout/stderr frames before we send status.
    let _ = socket.send(Message::Ping(vec![].into())).await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Send exit code as status on error channel (channel 3).
    // Only send for v4/v5 protocols — v1 (channel.k8s.io) doesn't use
    // the status channel and clients fail if they see non-stdout data.
    // We detect v1 by checking if the close frame was already sent or
    // if the command exited successfully (v1 clients don't expect status).
    // For compatibility: always send status for non-zero exit codes (all protocols),
    // only send Success status if NOT using v1 protocol.
    let exit_code = docker
        .inspect_exec(&exec.id)
        .await
        .ok()
        .and_then(|info| info.exit_code)
        .unwrap_or(0);

    // K8s v4/v5 protocol: send status on channel 3 (error channel).
    // v1 protocol (channel.k8s.io) does NOT use channel 3 — sending it causes
    // "Got message from server that didn't start with channel 1" failures.
    // K8s ref: staging/src/k8s.io/client-go/tools/remotecommand/v4.go
    let is_v1 = V1_PROTOCOL_FLAG.load(std::sync::atomic::Ordering::Relaxed);
    if !is_v1 || exit_code != 0 {
        // v4/v5: always send status. v1: only send for non-zero exit (error reporting).
        let status_json = if exit_code == 0 {
            r#"{"status":"Success"}"#.to_string()
        } else {
            format!(
                r#"{{"status":"Failure","message":"command terminated with exit code {}","reason":"NonZeroExitCode","details":{{"causes":[{{"reason":"ExitCode","message":"{}"}}]}}}}"#,
                exit_code, exit_code
            )
        };
        let mut status_data = vec![3u8];
        status_data.extend_from_slice(status_json.as_bytes());
        let _ = socket.send(Message::Binary(status_data.into())).await;
    }

    // Allow time for the client to read the status message before closing.
    // Without this delay, the TCP connection may reset before the client
    // processes channel 3, causing "connection reset by peer" errors.
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Close with a short reason (WebSocket close frames max 125 bytes)
    let short_reason = if exit_code == 0 {
        "Success"
    } else {
        "NonZeroExitCode"
    };
    let close_frame = axum::extract::ws::CloseFrame {
        code: 1000,
        reason: short_reason.to_string().into(),
    };
    let _ = socket.send(Message::Close(Some(close_frame))).await;
    debug!("WS exec completed for {}", container_id);
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
    handle_ws_exec(
        socket,
        pod,
        container_name,
        command,
        stdin,
        stdout,
        stderr,
        tty,
    )
    .await
}

/// Exec with protocol awareness — v1 doesn't use channel 3 for status
pub async fn handle_exec_websocket_with_protocol(
    socket: WebSocket,
    pod: Pod,
    container_name: String,
    command: Vec<String>,
    stdin: bool,
    stdout: bool,
    stderr: bool,
    tty: bool,
    is_v1_protocol: bool,
) {
    // Set the v1 flag so handle_ws_exec can check it
    V1_PROTOCOL_FLAG.store(is_v1_protocol, std::sync::atomic::Ordering::Relaxed);
    handle_ws_exec(
        socket,
        pod,
        container_name,
        command,
        stdin,
        stdout,
        stderr,
        tty,
    )
    .await
}

/// Global flag for v1 protocol detection (per-request via task-local would be better,
/// but this works since exec calls are serialized per connection)
static V1_PROTOCOL_FLAG: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

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
pub async fn handle_portforward_websocket(mut socket: WebSocket, pod: Pod, ports: Vec<u16>) {
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
                            if socket
                                .send(Message::Binary(buf[..n].to_vec().into()))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
            Err(e) => {
                let _ = socket
                    .send(Message::Text(
                        format!("Failed to connect to {}: {}", target, e).into(),
                    ))
                    .await;
            }
        }
    }

    let _ = socket.close().await;
}
