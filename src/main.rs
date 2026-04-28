mod backend;
mod cli;
mod config;
mod daemon;
mod device;
mod osd;

use anyhow::{Context, Result, bail};
use clap::Parser;
use cli::{Cli, Command};
use config::{Config, config_path};
use device::{DeviceInfo, list_event_devices, print_devices, print_probe};
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("wheelctl=info".parse()?))
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Devices => {
            let devices = list_event_devices()?;
            print_devices(&devices);
        }
        Command::Probe { path } => {
            let info = DeviceInfo::from_path(path)?;
            print_probe(&info);
        }
        Command::Add { path } => {
            let info = DeviceInfo::from_path(&path)?;
            let mut config = Config::load_or_default()?;
            let mut device = info.to_default_config();
            device.path = path.display().to_string();
            config.add_device(device)?;
            config.save()?;
            println!("Added '{}' to {}", info.name, config_path()?.display());
        }
        Command::Remove { key } => {
            let mut config = Config::load_or_default()?;
            if !config.remove_device(&key) {
                bail!("device key '{key}' was not found in config");
            }
            config.save()?;
            println!("Removed '{key}' from {}", config_path()?.display());
        }
        Command::Show => {
            let path = config_path()?;
            let config = Config::load_or_default()?;
            if !path.exists() {
                println!("No config found at {}.", path.display());
                println!("Parsed config is empty:");
            }
            let rendered = toml::to_string_pretty(&config).context("failed to render config")?;
            print!("{rendered}");
        }
        Command::Run => {
            let config = Config::load_or_default()?;
            daemon::run(config)?;
        }
    }

    Ok(())
}
