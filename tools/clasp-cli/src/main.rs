//! CLASP CLI tool
//!
//! A command-line interface for debugging, testing, and interacting
//! with CLASP servers and devices.

use anyhow::Result;
use clap::{Parser, Subcommand};
use clasp_client::Clasp;
use clasp_discovery::Discovery;
use std::time::Duration;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "clasp")]
#[command(about = "CLASP CLI - Debug and test CLASP connections")]
#[command(version)]
struct Cli {
    /// Server URL (default: ws://localhost:7330)
    #[arg(short, long, default_value = "ws://localhost:7330")]
    url: String,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Discover CLASP devices on the network
    Discover {
        /// Discovery timeout in seconds
        #[arg(short, long, default_value = "5")]
        timeout: u64,
    },

    /// Get a parameter value
    Get {
        /// Address to get (e.g., /lumen/layer/0/opacity)
        address: String,
    },

    /// Set a parameter value
    Set {
        /// Address to set
        address: String,
        /// Value (JSON format)
        value: String,
    },

    /// Subscribe to address pattern and print updates
    Watch {
        /// Address pattern (e.g., /lumen/layer/*/opacity)
        pattern: String,
    },

    /// Emit an event
    Emit {
        /// Event address
        address: String,
        /// Optional payload (JSON format)
        payload: Option<String>,
    },

    /// Send a stream of values
    Stream {
        /// Stream address
        address: String,
        /// Value (JSON format)
        value: String,
        /// Rate in Hz
        #[arg(short, long, default_value = "30")]
        rate: f64,
        /// Duration in seconds (0 = forever)
        #[arg(short, long, default_value = "0")]
        duration: u64,
    },

    /// Show server info
    Info,

    /// Interactive REPL mode
    Repl,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    match cli.command {
        Commands::Discover { timeout } => {
            println!("Discovering CLASP devices...");
            let discovery = Discovery::new().await?;
            discovery.start().await?;

            tokio::time::sleep(Duration::from_secs(timeout)).await;

            let devices = discovery.devices();
            if devices.is_empty() {
                println!("No devices found.");
            } else {
                println!("\nFound {} device(s):\n", devices.len());
                for device in devices {
                    println!("  {} ({})", device.name, device.id);
                    for endpoint in &device.endpoints {
                        println!("    - {}", endpoint);
                    }
                    println!();
                }
            }
        }

        Commands::Get { address } => {
            let client = Clasp::builder(&cli.url)
                .name("clasp-cli")
                .connect()
                .await?;

            match client.get(&address).await {
                Ok(value) => {
                    let json = serde_json::to_string_pretty(&value)?;
                    println!("{}", json);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                }
            }

            client.close().await?;
        }

        Commands::Set { address, value } => {
            let client = Clasp::builder(&cli.url)
                .name("clasp-cli")
                .connect()
                .await?;

            let parsed: serde_json::Value = serde_json::from_str(&value)?;
            client.set(&address, parsed.into()).await?;
            println!("Set {} = {}", address, value);

            client.close().await?;
        }

        Commands::Watch { pattern } => {
            let client = Clasp::builder(&cli.url)
                .name("clasp-cli")
                .connect()
                .await?;

            println!("Watching {}... (Ctrl+C to stop)\n", pattern);

            let _unsub = client.subscribe(&pattern, |value, addr| {
                let json = serde_json::to_string(&value).unwrap_or_default();
                println!("{} = {}", addr, json);
            }).await?;

            // Wait forever
            tokio::signal::ctrl_c().await?;
            client.close().await?;
        }

        Commands::Emit { address, payload } => {
            let client = Clasp::builder(&cli.url)
                .name("clasp-cli")
                .connect()
                .await?;

            let payload_value = if let Some(p) = payload {
                let parsed: serde_json::Value = serde_json::from_str(&p)?;
                Some(parsed.into())
            } else {
                None
            };

            client.emit(&address, payload_value).await?;
            println!("Emitted event: {}", address);

            client.close().await?;
        }

        Commands::Stream { address, value, rate, duration } => {
            let client = Clasp::builder(&cli.url)
                .name("clasp-cli")
                .connect()
                .await?;

            let parsed: serde_json::Value = serde_json::from_str(&value)?;
            let interval = Duration::from_secs_f64(1.0 / rate);
            let end_time = if duration > 0 {
                Some(std::time::Instant::now() + Duration::from_secs(duration))
            } else {
                None
            };

            println!("Streaming to {} at {} Hz... (Ctrl+C to stop)", address, rate);

            loop {
                if let Some(end) = end_time {
                    if std::time::Instant::now() >= end {
                        break;
                    }
                }

                client.stream(&address, parsed.clone().into()).await?;
                tokio::time::sleep(interval).await;
            }

            client.close().await?;
        }

        Commands::Info => {
            let client = Clasp::builder(&cli.url)
                .name("clasp-cli")
                .connect()
                .await?;

            println!("Connected to: {}", cli.url);
            println!("Session ID: {}", client.session_id().unwrap_or_default());
            println!("Server time: {} µs", client.time());

            client.close().await?;
        }

        Commands::Repl => {
            println!("CLASP REPL");
            println!("Commands: get <addr>, set <addr> <value>, watch <pattern>, emit <addr> [payload], quit");
            println!();

            let client = Clasp::builder(&cli.url)
                .name("clasp-repl")
                .connect()
                .await?;

            let stdin = std::io::stdin();
            let mut line = String::new();

            loop {
                print!("clasp> ");
                use std::io::Write;
                std::io::stdout().flush()?;

                line.clear();
                if stdin.read_line(&mut line)? == 0 {
                    break;
                }

                let parts: Vec<&str> = line.trim().split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }

                match parts[0] {
                    "quit" | "exit" | "q" => break,
                    "get" if parts.len() >= 2 => {
                        match client.get(parts[1]).await {
                            Ok(value) => println!("{:?}", value),
                            Err(e) => eprintln!("Error: {}", e),
                        }
                    }
                    "set" if parts.len() >= 3 => {
                        let value_str = parts[2..].join(" ");
                        match serde_json::from_str::<serde_json::Value>(&value_str) {
                            Ok(v) => {
                                if let Err(e) = client.set(parts[1], v.into()).await {
                                    eprintln!("Error: {}", e);
                                } else {
                                    println!("OK");
                                }
                            }
                            Err(e) => eprintln!("Invalid JSON: {}", e),
                        }
                    }
                    "emit" if parts.len() >= 2 => {
                        let payload = if parts.len() >= 3 {
                            let value_str = parts[2..].join(" ");
                            serde_json::from_str::<serde_json::Value>(&value_str).ok().map(Into::into)
                        } else {
                            None
                        };
                        if let Err(e) = client.emit(parts[1], payload).await {
                            eprintln!("Error: {}", e);
                        } else {
                            println!("OK");
                        }
                    }
                    "help" | "?" => {
                        println!("Commands:");
                        println!("  get <address>           - Get parameter value");
                        println!("  set <address> <json>    - Set parameter value");
                        println!("  emit <address> [json]   - Emit event");
                        println!("  quit                    - Exit REPL");
                    }
                    _ => println!("Unknown command. Type 'help' for available commands."),
                }
            }

            client.close().await?;
        }
    }

    Ok(())
}
