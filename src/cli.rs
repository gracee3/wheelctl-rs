use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about = "Turn a grabbed Linux evdev wheel device into a control wheel"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List candidate /dev/input/event* devices.
    Devices,
    /// Print evdev metadata and capabilities for a device.
    Probe { path: PathBuf },
    /// Print key and wheel events from a device until interrupted.
    Events {
        /// Also print REL_X and REL_Y pointer movement events.
        #[arg(long)]
        movement: bool,
        /// Do not exclusively grab the device while reading events.
        #[arg(long)]
        no_grab: bool,
        path: PathBuf,
    },
    /// Probe a device and append a default config stanza.
    Add { path: PathBuf },
    /// Remove a configured device by key.
    Remove { key: String },
    /// Print the parsed configuration.
    Show,
    /// Send a test OSD notification using the configured OSD backend.
    OsdTest,
    /// Run in the foreground and process configured wheel devices.
    Run,
}
