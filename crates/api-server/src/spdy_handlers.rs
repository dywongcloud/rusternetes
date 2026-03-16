//! SPDY handlers for pod exec, attach, and port-forward
//!
//! Implements kubectl-compatible SPDY protocol handlers

use crate::spdy::{SpdyChannel, SpdyConnection};
use anyhow::{Context, Result};
use futures::StreamExt;
use rusternetes_common::resources::Pod;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// Handle SPDY exec connection
pub async fn handle_spdy_exec(
    spdy: SpdyConnection,
    pod: Pod,
    container_name: String,
    command: Vec<String>,
    stdin: bool,
    stdout: bool,
    stderr: bool,
    tty: bool,
) {
    debug!(
        "SPDY exec: pod={}, container={}, command={:?}, stdin={}, stdout={}, stderr={}, tty={}",
        pod.metadata.name, container_name, command, stdin, stdout, stderr, tty
    );

    // Get the container runtime command
    let container_id = format!("{}_{}", pod.metadata.name, container_name);

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

            let spdy = Arc::new(spdy);

            // Handle stdin if requested
            if stdin {
                if let Some(mut child_stdin) = child.stdin.take() {
                    let spdy_clone = Arc::clone(&spdy);
                    tokio::spawn(async move {
                        loop {
                            match spdy_clone.read_frame().await {
                                Ok(Some(frame)) if frame.channel == SpdyChannel::Stdin => {
                                    if let Err(e) = child_stdin.write_all(&frame.data).await {
                                        error!("Failed to write to stdin: {}", e);
                                        break;
                                    }
                                }
                                Ok(Some(frame)) if frame.channel == SpdyChannel::Resize => {
                                    // Terminal resize - could implement terminal size changes here
                                    debug!("Received terminal resize: {:?}", frame.data);
                                }
                                Ok(None) => {
                                    // Connection closed
                                    debug!("SPDY connection closed for stdin");
                                    break;
                                }
                                Err(e) => {
                                    error!("Error reading SPDY frame: {}", e);
                                    break;
                                }
                                _ => {}
                            }
                        }
                    });
                }
            }

            // Handle stdout if requested
            if stdout {
                if let Some(mut child_stdout) = child.stdout.take() {
                    let spdy_clone = Arc::clone(&spdy);
                    tokio::spawn(async move {
                        let mut buffer = vec![0u8; 8192];
                        loop {
                            match child_stdout.read(&mut buffer).await {
                                Ok(0) => break, // EOF
                                Ok(n) => {
                                    let data = buffer[..n].to_vec();
                                    if let Err(e) =
                                        spdy_clone.write_channel(SpdyChannel::Stdout, data).await
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
                    let spdy_clone = Arc::clone(&spdy);
                    tokio::spawn(async move {
                        let mut buffer = vec![0u8; 8192];
                        loop {
                            match child_stderr.read(&mut buffer).await {
                                Ok(0) => break, // EOF
                                Ok(n) => {
                                    let data = buffer[..n].to_vec();
                                    if let Err(e) =
                                        spdy_clone.write_channel(SpdyChannel::Stderr, data).await
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
                    let _ = spdy.close().await;
                }
                Err(e) => {
                    error!("Exec command failed: {}", e);
                    let _ = spdy.write_error(&format!("Command failed: {}", e)).await;
                    let _ = spdy.close().await;
                }
            }
        }
        Err(e) => {
            error!("Failed to spawn exec command: {}", e);
            let _ = spdy
                .write_error(&format!("Failed to execute command: {}", e))
                .await;
            let _ = spdy.close().await;
        }
    }
}

/// Handle SPDY attach connection
pub async fn handle_spdy_attach(
    spdy: SpdyConnection,
    pod: Pod,
    container_name: String,
    stdin: bool,
    stdout: bool,
    stderr: bool,
    tty: bool,
) {
    debug!(
        "SPDY attach: pod={}, container={}, stdin={}, stdout={}, stderr={}, tty={}",
        pod.metadata.name, container_name, stdin, stdout, stderr, tty
    );

    let container_id = format!("{}_{}", pod.metadata.name, container_name);

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

            let spdy = Arc::new(spdy);

            // Handle stdin if requested
            if stdin {
                if let Some(mut child_stdin) = child.stdin.take() {
                    let spdy_clone = Arc::clone(&spdy);
                    tokio::spawn(async move {
                        loop {
                            match spdy_clone.read_frame().await {
                                Ok(Some(frame)) if frame.channel == SpdyChannel::Stdin => {
                                    if let Err(e) = child_stdin.write_all(&frame.data).await {
                                        error!("Failed to write to stdin: {}", e);
                                        break;
                                    }
                                }
                                Ok(Some(frame)) if frame.channel == SpdyChannel::Resize => {
                                    debug!("Received terminal resize: {:?}", frame.data);
                                }
                                Ok(None) => {
                                    debug!("SPDY connection closed for stdin");
                                    break;
                                }
                                Err(e) => {
                                    error!("Error reading SPDY frame: {}", e);
                                    break;
                                }
                                _ => {}
                            }
                        }
                    });
                }
            }

            // Handle stdout if requested
            if stdout {
                if let Some(mut child_stdout) = child.stdout.take() {
                    let spdy_clone = Arc::clone(&spdy);
                    tokio::spawn(async move {
                        let mut buffer = vec![0u8; 8192];
                        loop {
                            match child_stdout.read(&mut buffer).await {
                                Ok(0) => break,
                                Ok(n) => {
                                    if let Err(e) = spdy_clone
                                        .write_channel(SpdyChannel::Stdout, buffer[..n].to_vec())
                                        .await
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
                    let spdy_clone = Arc::clone(&spdy);
                    tokio::spawn(async move {
                        let mut buffer = vec![0u8; 8192];
                        loop {
                            match child_stderr.read(&mut buffer).await {
                                Ok(0) => break,
                                Ok(n) => {
                                    if let Err(e) = spdy_clone
                                        .write_channel(SpdyChannel::Stderr, buffer[..n].to_vec())
                                        .await
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
                    let _ = spdy.close().await;
                }
                Err(e) => {
                    error!("Attach failed: {}", e);
                    let _ = spdy.write_error(&format!("Attach failed: {}", e)).await;
                    let _ = spdy.close().await;
                }
            }
        }
        Err(e) => {
            error!("Failed to attach to container: {}", e);
            let _ = spdy.write_error(&format!("Failed to attach: {}", e)).await;
            let _ = spdy.close().await;
        }
    }
}

/// Handle SPDY port-forward connection
pub async fn handle_spdy_portforward(spdy: SpdyConnection, pod: Pod, ports: Vec<u16>) {
    debug!(
        "SPDY port-forward: pod={}, ports={:?}",
        pod.metadata.name, ports
    );

    info!(
        "Port-forward for pod {} on ports {:?} - establishing TCP proxies",
        pod.metadata.name, ports
    );

    // Get pod IP from status
    let pod_ip = match &pod.status {
        Some(status) => match &status.pod_ip {
            Some(ip) => ip.clone(),
            None => {
                let _ = spdy.write_error("Pod has no IP address assigned").await;
                let _ = spdy.close().await;
                return;
            }
        },
        None => {
            let _ = spdy.write_error("Pod has no status").await;
            let _ = spdy.close().await;
            return;
        }
    };

    // For each port, we need to:
    // 1. Read data from SPDY stream for that port
    // 2. Forward to pod_ip:port via TCP
    // 3. Read response from TCP and write back to SPDY stream
    //
    // SPDY port-forward uses pairs of streams per port:
    // - Data stream (even numbered starting from 0)
    // - Error stream (odd numbered starting from 1)
    //
    // For simplicity in this reference implementation, we'll handle one port at a time
    // and use a simplified multiplexing scheme

    let spdy = Arc::new(spdy);

    for port in ports {
        let pod_ip = pod_ip.clone();
        let spdy_clone = Arc::clone(&spdy);

        tokio::spawn(async move {
            match setup_port_forward(spdy_clone, &pod_ip, port).await {
                Ok(_) => {
                    info!("Port-forward for port {} completed successfully", port);
                }
                Err(e) => {
                    error!("Port-forward for port {} failed: {}", port, e);
                }
            }
        });
    }

    // Keep connection alive until all port forwards are done
    // In practice, we'd track active port forwards and close when all are done
    // For now, just wait for connection close
    loop {
        match spdy.read_frame().await {
            Ok(None) => {
                info!("SPDY port-forward connection closed");
                break;
            }
            Ok(Some(_)) => {
                // Handle frame (port-forward data)
                // This would route to appropriate TCP connection
            }
            Err(e) => {
                error!("Error reading SPDY frame: {}", e);
                break;
            }
        }
    }

    let _ = spdy.close().await;
}

/// Set up TCP proxy for a single port
async fn setup_port_forward(spdy: Arc<SpdyConnection>, pod_ip: &str, port: u16) -> Result<()> {
    use tokio::net::TcpStream;

    info!("Setting up port-forward to {}:{}", pod_ip, port);

    // Connect to pod
    let target_addr = format!("{}:{}", pod_ip, port);
    let tcp_stream = TcpStream::connect(&target_addr)
        .await
        .context(format!("Failed to connect to {}", target_addr))?;

    let (mut tcp_read, mut tcp_write) = tcp_stream.into_split();

    // Spawn task to forward SPDY -> TCP
    let spdy_to_tcp = Arc::clone(&spdy);
    tokio::spawn(async move {
        loop {
            match spdy_to_tcp.read_frame().await {
                Ok(Some(frame)) if frame.channel == SpdyChannel::Stdin => {
                    // Data from client to pod
                    if let Err(e) = tcp_write.write_all(&frame.data).await {
                        error!("Failed to write to TCP connection: {}", e);
                        break;
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    error!("Error reading SPDY frame: {}", e);
                    break;
                }
                _ => {}
            }
        }
    });

    // Forward TCP -> SPDY
    let mut buffer = vec![0u8; 8192];
    loop {
        match tcp_read.read(&mut buffer).await {
            Ok(0) => break, // EOF
            Ok(n) => {
                if let Err(e) = spdy
                    .write_channel(SpdyChannel::Stdout, buffer[..n].to_vec())
                    .await
                {
                    error!("Failed to write to SPDY connection: {}", e);
                    break;
                }
            }
            Err(e) => {
                error!("Failed to read from TCP connection: {}", e);
                break;
            }
        }
    }

    Ok(())
}
