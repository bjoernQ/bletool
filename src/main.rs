use std::{
    error::Error,
    time::{Duration, SystemTime},
};

use bluest::{Adapter, Uuid};
use clap::{Args, Parser, Subcommand};
use futures_lite::StreamExt;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Write(WriteArgs),
}

#[derive(Debug, Args)]
pub struct WriteArgs {
    name: String,
    service_uuid: String,
    characteristic: String,
    what: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Write(args) => {
            let adapter = Adapter::default()
                .await
                .ok_or("Bluetooth adapter not found")?;
            adapter.wait_available().await?;

            let now = SystemTime::now();
            let mut device = None;
            let mut scan = adapter.scan(&[]).await?;
            while let Some(discovered_device) = scan.next().await {
                println!(
                    "{}{}: {:?}",
                    discovered_device
                        .device
                        .name()
                        .as_deref()
                        .unwrap_or("(unknown)"),
                    discovered_device
                        .rssi
                        .map(|x| format!(" ({}dBm)", x))
                        .unwrap_or_default(),
                    discovered_device.adv_data.services
                );

                if discovered_device.device.name()? == args.name {
                    device = Some(discovered_device.device);
                    break;
                }

                if now.elapsed()? > Duration::from_secs(15) {
                    return Ok(());
                }
            }

            if let Some(device) = device {
                adapter.connect_device(&device).await?;
                let services = device.discover_services().await?;

                for service in services {
                    if service.uuid() == Uuid::parse_str(&args.service_uuid)? {
                        let characteristics = service.characteristics().await?;
                        for c in characteristics {
                            if c.uuid() == Uuid::parse_str(&args.characteristic)? {
                                c.write(args.what.as_bytes()).await?;
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
