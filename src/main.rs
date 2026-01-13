//! WebSocket Ping-Pong Benchmark in Rust
//! Rewrite of C benchmark with support for TLS

mod benchmark;
mod cli;
mod stats;
mod utils;
mod websocket;

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use clap::Parser;

// Global configuration
pub static QUIET_MODE: AtomicBool = AtomicBool::new(false);

use cli::Args;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Set quiet mode globally
    QUIET_MODE.store(args.quiet, Ordering::SeqCst);

    // Create async runtime
    let rt = tokio::runtime::Runtime::new()?;

    rt.block_on(async {
        cli::run_interactive_or_command(args).await
    })
}
