use crate::stats::{CpuTime, RttStats};
use crate::utils::generate_websocket_key;
use crate::websocket::WebSocketFrame;
use crate::QUIET_MODE;
use anyhow::{Context, Result};
use native_tls::TlsConnector;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::Ordering;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use futures_util::{sink::SinkExt, stream::StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

const PING_MESSAGE: &[u8] = b"PING";
const BUFFER_SIZE: usize = 4096;

/// Benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub host: String,
    pub port: u16,
    pub ping_pong_count: u32,
    pub quiet: bool,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        BenchmarkConfig {
            host: "10.25.96.5".to_string(),
            port: 8443,
            ping_pong_count: 30,
            quiet: false,
        }
    }
}

/// Print benchmark header
pub fn print_benchmark_header(name: &str) {
    println!("┌──────────────────────────────────────────────────────────────┐");
    println!("│ Benchmark: {:50}│", name);
    println!("├──────────────────────────────────────────────────────────────┤");
}

/// Print benchmark result (only if logging is enabled)
pub fn print_benchmark_result(iteration: u32, rtt_ms: f64) {
    if !QUIET_MODE.load(Ordering::SeqCst) {
        println!("│ Ping-Pong {:>6} RTT: {:>10.3} ms                          │", iteration, rtt_ms);
    }
}

/// Benchmark 1: Basic OpenSSL TLS 1.3 with blocking I/O
pub async fn run_openssl_benchmark(config: &BenchmarkConfig) -> Result<()> {
    print_benchmark_header("Rust socket + Native TLS (blocking I/O)");

    let mut cpu = CpuTime::new();
    cpu.start();

    let url = format!("wss://{}:{}/ws", config.host, config.port);
    let (ws_stream, _) = connect_async(&url).await.context("Failed to connect")?;

    let mut ws_stream = ws_stream;

    let mut rtts = Vec::with_capacity(config.ping_pong_count as usize);

    for i in 0..config.ping_pong_count {
        let start = Instant::now();

        // Send PING
        let ping_frame = WebSocketFrame::create_text_frame(PING_MESSAGE);
        ws_stream
            .send(Message::Binary(ping_frame))
            .await
            .context("Failed to send PING")?;

        // Receive PONG
        let msg = ws_stream
            .next()
            .await
            .context("Connection closed")?
            .context("Failed to receive PONG")?;

        let end = start.elapsed().as_millis() as f64;
        rtts.push(end);

        // Parse frame if needed
        if let Message::Binary(data) = msg {
            let _frame = WebSocketFrame::parse_frame(&data);
        }

        print_benchmark_result(i + 1, end);
    }

    cpu.stop();
    let stats = RttStats::new(rtts.clone());
    stats.print_rtt_stats();
    RttStats::print_cpu_time(&cpu, stats.count);
    println!("└──────────────────────────────────────────────────────────────┘");

    Ok(())
}

/// Benchmark 2: TLS with async wait (tokio-based)
pub async fn run_async_benchmark(config: &BenchmarkConfig) -> Result<()> {
    print_benchmark_header("Rust async + Native TLS (tokio)");

    let mut cpu = CpuTime::new();
    cpu.start();

    let url = format!("wss://{}:{}/ws", config.host, config.port);
    let (mut ws_stream, _) = connect_async(&url).await?;

    let mut rtts = Vec::with_capacity(config.ping_pong_count as usize);

    for i in 0..config.ping_pong_count {
        let start = Instant::now();

        // Send PING
        let ping_frame = WebSocketFrame::create_text_frame(PING_MESSAGE);
        ws_stream.send(Message::Binary(ping_frame)).await?;

        // Receive PONG (async wait)
        let msg = ws_stream.next().await.ok_or_else(|| anyhow::anyhow!("Connection closed"))??;

        let end = start.elapsed().as_millis() as f64;
        rtts.push(end);

        if let Message::Binary(data) = msg {
            let _frame = WebSocketFrame::parse_frame(&data);
        }

        print_benchmark_result(i + 1, end);
    }

    cpu.stop();
    let stats = RttStats::new(rtts.clone());
    stats.print_rtt_stats();
    RttStats::print_cpu_time(&cpu, stats.count);
    println!("└──────────────────────────────────────────────────────────────┘");

    Ok(())
}

/// Benchmark 11: Basic TLS (similar to C's OpenSSL benchmark)
pub fn run_basic_tls_benchmark_sync(config: &BenchmarkConfig) -> Result<()> {
    print_benchmark_header("Rust sync socket + Native TLS");

    let mut cpu = CpuTime::new();
    cpu.start();

    // Connect TCP
    let tcp_stream = TcpStream::connect((config.host.as_str(), config.port))
        .context("Failed to connect TCP")?;

    // Set TCP_NODELAY
    tcp_stream.set_nodelay(true)?;

    // Create TLS connector
    let connector = TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .danger_accept_invalid_hostnames(true)
        .build()
        .context("Failed to create TLS connector")?;

    let mut tls_stream = connector
        .connect(&config.host, tcp_stream)
        .context("Failed to connect TLS")?;

    // WebSocket handshake
    let ws_key = generate_websocket_key();
    let request = format!(
        "GET /ws HTTP/1.1\r\n\
         Host: {}:{}\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Key: {}\r\n\
         Sec-WebSocket-Version: 13\r\n\
         \r\n",
        config.host, config.port, ws_key
    );

    tls_stream.write_all(request.as_bytes())?;
    tls_stream.flush()?;

    // Read handshake response
    let mut response = vec![0u8; BUFFER_SIZE];
    let bytes_read = tls_stream.read(&mut response)?;
    let response = String::from_utf8_lossy(&response[..bytes_read]);

    if !response.contains("101") {
        return Err(anyhow::anyhow!("WebSocket handshake failed"));
    }

    // Run ping-pong
    let mut rtts = Vec::with_capacity(config.ping_pong_count as usize);
    let mut recv_buf = vec![0u8; BUFFER_SIZE];

    for i in 0..config.ping_pong_count {
        let start = Instant::now();

        // Send PING
        let ping_frame = WebSocketFrame::create_text_frame(PING_MESSAGE);
        tls_stream.write_all(&ping_frame)?;
        tls_stream.flush()?;

        // Receive PONG
        let bytes_read = tls_stream.read(&mut recv_buf)?;
        let _frame = WebSocketFrame::parse_frame(&recv_buf[..bytes_read]);

        let end = start.elapsed().as_millis() as f64;
        rtts.push(end);

        print_benchmark_result(i + 1, end);
    }

    cpu.stop();
    let stats = RttStats::new(rtts.clone());
    stats.print_rtt_stats();
    RttStats::print_cpu_time(&cpu, stats.count);
    println!("└──────────────────────────────────────────────────────────────┘");

    Ok(())
}

/// Benchmark 6: Multi-connection test (simplified version)
pub async fn run_multi_connection_benchmark(config: &BenchmarkConfig) -> Result<()> {
    println!("┌──────────────────────────────────────────────────────────────┐");
    println!("│ Multi-Connection Benchmark (50 clients × {} ping-pong)          │", config.ping_pong_count);
    println!("├──────────────────────────────────────────────────────────────┤");
    println!("│ Implementation: Rust async + Native TLS                      │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let mut cpu = CpuTime::new();
    cpu.start();

    let client_count = 50;
    let url = format!("wss://{}:{}/ws", config.host, config.port);

    let mut handles = Vec::new();

    for _ in 0..client_count {
        let url_clone = url.clone();
        let count = config.ping_pong_count;

        let handle = tokio::spawn(async move {
            let mut local_rtts = Vec::with_capacity(count as usize);

                match connect_async(&url_clone).await {
                    Ok((mut ws_stream, _)) => {
                        for _ in 0..count {
                            let start = Instant::now();

                            let ping_frame = WebSocketFrame::create_text_frame(PING_MESSAGE);
                            if ws_stream.send(Message::Binary(ping_frame)).await.is_ok() {
                                if let Some(Ok(msg)) = ws_stream.next().await {
                                    if let Message::Binary(_) = msg {
                                        local_rtts.push(start.elapsed().as_millis() as f64);
                                    }
                                }
                            }
                        }
                        Some(local_rtts)
                    }
                    Err(_) => None,
                }
        });

        handles.push(handle);
    }

    let mut all_rtts = Vec::new();

    for handle in handles {
        if let Ok(Some(rtts)) = handle.await {
            all_rtts.extend(rtts);
        }
    }

    cpu.stop();

    if !all_rtts.is_empty() {
        let stats = RttStats::new(all_rtts);
        let throughput = if cpu.wall_time > 0.0 {
            stats.count as f64 / cpu.wall_time
        } else {
            0.0
        };

        println!("┌──────────────────────────────────────────────────────────────┐");
        println!("│ Results: {} clients × {} ping-pong = {} messages           │",
            client_count, config.ping_pong_count, stats.count);
        println!("├──────────────────────────────────────────────────────────────┤");
        println!("│ Total Time: {:7.2}s | Throughput: {:8.0} msg/sec         │",
            cpu.wall_time, throughput);
        println!("│ Avg RTT: {:7.3} ms | Median: {:7.3} ms                   │",
            stats.avg, stats.median);
        println!("│ Min RTT: {:7.3} ms | Max: {:7.3} ms                      │",
            stats.min, stats.max);
        println!("└──────────────────────────────────────────────────────────────┘");
    }

    Ok(())
}

/// TCP benchmark (no TLS)
pub async fn run_tcp_benchmark(config: &BenchmarkConfig) -> Result<()> {
    print_benchmark_header("Rust TCP (no TLS)");

    let mut cpu = CpuTime::new();
    cpu.start();

    let mut socket = tokio::net::TcpStream::connect((config.host.as_str(), config.port))
        .await
        .context("Failed to connect")?;

    let mut rtts = Vec::with_capacity(config.ping_pong_count as usize);

    for i in 0..config.ping_pong_count {
        let start = Instant::now();

        socket.write_all(PING_MESSAGE).await?;

        let mut buf = vec![0u8; BUFFER_SIZE];
        let n = socket.read(&mut buf).await?;

        let end = start.elapsed().as_millis() as f64;
        if n > 0 {
            rtts.push(end);
        }

        print_benchmark_result(i + 1, end);
    }

    cpu.stop();
    let stats = RttStats::new(rtts.clone());
    stats.print_rtt_stats();
    RttStats::print_cpu_time(&cpu, stats.count);
    println!("└──────────────────────────────────────────────────────────────┘");

    Ok(())
}

/// UDP benchmark
pub async fn run_udp_benchmark(config: &BenchmarkConfig) -> Result<()> {
    print_benchmark_header("Rust UDP");

    let mut cpu = CpuTime::new();
    cpu.start();

    let socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;
    let udp_port = config.port + 2; // Use port 8445 for UDP

    let mut rtts = Vec::with_capacity(config.ping_pong_count as usize);

    for i in 0..config.ping_pong_count {
        let start = Instant::now();

        socket.send_to(PING_MESSAGE, (config.host.as_str(), udp_port)).await?;

        let mut buf = vec![0u8; BUFFER_SIZE];
        let n = socket.recv(&mut buf).await?;

        let end = start.elapsed().as_millis() as f64;
        if n > 0 {
            rtts.push(end);
        }

        print_benchmark_result(i + 1, end);
    }

    cpu.stop();
    let stats = RttStats::new(rtts.clone());
    stats.print_rtt_stats();
    RttStats::print_cpu_time(&cpu, stats.count);
    println!("└──────────────────────────────────────────────────────────────┘");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_config_default() {
        let config = BenchmarkConfig::default();
        assert_eq!(config.host, "10.25.96.5");
        assert_eq!(config.port, 8443);
        assert_eq!(config.ping_pong_count, 30);
    }
}
