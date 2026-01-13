use crate::benchmark::{self, BenchmarkConfig};
use crate::QUIET_MODE;
use anyhow::Result;
use clap::Parser;
use std::io::{self, Write};
use std::sync::atomic::Ordering;

/// WebSocket Ping-Pong Benchmark CLI
#[derive(Parser, Debug)]
#[command(name = "websocket_benchmark")]
#[command(author = "Benchmark Team")]
#[command(version = "1.0")]
#[command(about = "WebSocket Ping-Pong Benchmark in Rust", long_about = None)]
pub struct Args {
    /// Benchmark number to run (1-19)
    #[arg(short, long, value_name = "NUM")]
    pub benchmark: Option<u8>,

    /// Server hostname
    #[arg(short, long, default_value = "10.25.96.5", value_name = "HOST")]
    pub host: String,

    /// Server port
    #[arg(short, long, default_value_t = 8443, value_name = "PORT")]
    pub port: u16,

    /// Number of ping-pong iterations
    #[arg(short, long, default_value_t = 30, value_name = "COUNT")]
    pub count: u32,

    /// Quiet mode (disable per-iteration logging)
    #[arg(short, long)]
    pub quiet: bool,

    /// Run multi-connection test
    #[arg(long)]
    pub multi: bool,
}

fn print_header() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║   Rust WebSocket Ping-Pong Benchmark (TLS/WSS)               ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Connecting to wss://10.25.96.5:8443                        ║");
    println!("║  Make sure WebSocket server is running!                      ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
}

fn show_menu(config: &BenchmarkConfig) {
    println!("═══════════════════════════════════════════════════════════════");
    println!("  Rust WebSocket Benchmark (TLS only, wss://{}:{})", config.host, config.port);
    println!("  {} ping-pong iterations with RTT measurement", config.ping_pong_count);
    println!("═══════════════════════════════════════════════════════════════");
    println!();
    println!("  === Native TLS (userspace TLS) ===");
    println!("  1. async + Native TLS (tokio)");
    println!("  2. sync + Native TLS (blocking I/O)");
    println!();
    println!("  === Run All Benchmarks ===");
    println!("  5. Run ALL TLS benchmarks (1-2)");
    println!();
    println!("  === Multi-Connection ===");
    println!("  6. Multi-Connection (50 clients × {} ping-pong)", config.ping_pong_count);
    println!();
    println!("  === Other ===");
    println!("  7. TCP benchmark (no TLS)");
    println!("  8. UDP benchmark");
    println!();
    println!("  97. Toggle ping-pong logging ({})", if QUIET_MODE.load(Ordering::SeqCst) { "OFF" } else { "ON" });
    println!("  98. Change server address (current: {}:{})", config.host, config.port);
    println!("  99. Change ping-pong count (current: {})", config.ping_pong_count);
    println!();
    println!("  0. Exit");
    println!("═══════════════════════════════════════════════════════════════");
    print!("Enter choice: ");
    io::stdout().flush().unwrap();
}

fn read_line() -> String {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn ask_server_settings(config: &mut BenchmarkConfig) {
    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Configure WebSocket Server Address                          ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    print!("Server host [{}]: ", config.host);
    io::stdout().flush().unwrap();
    let input = read_line();
    if !input.is_empty() {
        config.host = input;
    }

    print!("Server port [{}]: ", config.port);
    io::stdout().flush().unwrap();
    let input = read_line();
    if !input.is_empty() {
        if let Ok(port) = input.parse::<u16>() {
            config.port = port;
        }
    }

    println!("Server set to: wss://{}:{}", config.host, config.port);
}

fn ask_ping_pong_count(config: &mut BenchmarkConfig) {
    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Enter number of ping-pong iterations (default: 30):       ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    print!("Ping-pong count [{}]: ", config.ping_pong_count);
    io::stdout().flush().unwrap();
    let input = read_line();
    if !input.is_empty() {
        if let Ok(count) = input.parse::<u32>() {
            if count > 0 && count <= 1_000_000 {
                config.ping_pong_count = count;
                println!("Ping-pong count set to: {}", config.ping_pong_count);
            } else {
                println!("Invalid value, using current: {}", config.ping_pong_count);
            }
        }
    }
}

async fn run_benchmark(num: u8, config: &BenchmarkConfig) -> Result<()> {
    match num {
        1 => benchmark::run_openssl_benchmark(config).await,
        2 => {
            let result = benchmark::run_basic_tls_benchmark_sync(config);
            // Convert sync result to async result
            result.map_err(|e| anyhow::anyhow!("{:?}", e))
        }
        5 => {
            benchmark::run_openssl_benchmark(config).await?;
            println!();
            benchmark::run_async_benchmark(config).await
        }
        6 => benchmark::run_multi_connection_benchmark(config).await,
        7 => benchmark::run_tcp_benchmark(config).await,
        8 => benchmark::run_udp_benchmark(config).await,
        _ => Err(anyhow::anyhow!("Unknown benchmark: {}", num)),
    }
}

pub async fn run_interactive_or_command(args: Args) -> Result<()> {
    print_header();

    let mut config = BenchmarkConfig {
        host: args.host.clone(),
        port: args.port,
        ping_pong_count: args.count,
        quiet: args.quiet,
    };

    // Set quiet mode globally
    QUIET_MODE.store(args.quiet, Ordering::SeqCst);

    // If benchmark is specified, run it and exit
    if let Some(benchmark_num) = args.benchmark {
        return run_benchmark(benchmark_num, &config).await;
    }

    // Interactive mode
    loop {
        show_menu(&config);
        let choice = read_line();

        match choice.as_str() {
            "0" | "exit" => {
                println!("Exiting...");
                break;
            }
            "97" => {
                let current = QUIET_MODE.load(Ordering::SeqCst);
                QUIET_MODE.store(!current, Ordering::SeqCst);
                println!("Ping-pong logging: {}", if !QUIET_MODE.load(Ordering::SeqCst) { "ENABLED" } else { "DISABLED" });
            }
            "98" => {
                ask_server_settings(&mut config);
            }
            "99" => {
                ask_ping_pong_count(&mut config);
            }
            num => {
                if let Ok(benchmark_num) = num.parse::<u8>() {
                    if let Err(e) = run_benchmark(benchmark_num, &config).await {
                        eprintln!("Error running benchmark {}: {:?}", benchmark_num, e);
                    }
                    println!();
                    println!("Press Enter to continue...");
                    read_line();
                } else {
                    println!("Invalid choice. Please try again.");
                }
            }
        }
        println!();
    }

    Ok(())
}
