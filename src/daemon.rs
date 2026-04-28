use crate::backend::change_volume;
use crate::config::{Config, DeviceConfig};
use crate::device::is_vertical_wheel;
use anyhow::{Context, Result};
use evdev::Device;
use std::thread;
use tracing::{error, info, warn};

pub fn run(config: Config) -> Result<()> {
    let enabled_devices: Vec<DeviceConfig> = config
        .devices
        .into_iter()
        .filter(|device| device.enabled)
        .collect();

    if enabled_devices.is_empty() {
        warn!("no enabled devices configured");
        return Ok(());
    }

    let mut handles = Vec::new();
    for device_config in enabled_devices {
        let name = device_config.name.clone();
        handles.push(thread::spawn(move || {
            if let Err(error) = run_device(device_config) {
                error!(device = %name, error = %error, "device worker stopped");
            }
        }));
    }

    for handle in handles {
        if handle.join().is_err() {
            error!("device worker panicked");
        }
    }

    Ok(())
}

fn run_device(config: DeviceConfig) -> Result<()> {
    let mut device = Device::open(&config.path)
        .with_context(|| format!("failed to open configured device {}", config.path))?;

    if config.grab {
        device
            .grab()
            .with_context(|| format!("failed to grab {}", config.path))?;
        info!(device = %config.name, path = %config.path, "grabbed device");
    } else {
        warn!(
            device = %config.name,
            "grab is disabled; unmapped events may still reach the desktop"
        );
    }

    info!(
        device = %config.name,
        path = %config.path,
        step = %config.mappings.scroll_vertical.step,
        "processing input events"
    );

    loop {
        for event in device
            .fetch_events()
            .context("failed to read evdev events")?
        {
            if is_vertical_wheel(event.event_type(), event.code()) {
                handle_vertical_scroll(&config, event.value())?;
                continue;
            }

            // Reading from a grabbed device and intentionally doing nothing here
            // suppresses pointer movement, buttons, and other unmapped events.
        }
    }
}

fn handle_vertical_scroll(config: &DeviceConfig, value: i32) -> Result<()> {
    if value == 0 || !config.mappings.scroll_vertical.enabled {
        return Ok(());
    }

    let mapping = &config.mappings.scroll_vertical;
    if mapping.target != "volume" {
        warn!(
            device = %config.name,
            target = %mapping.target,
            "unsupported scroll target"
        );
        return Ok(());
    }

    let increase = value > 0;
    for _ in 0..value.unsigned_abs() {
        change_volume(mapping.backend, &mapping.step, increase)?;
    }

    Ok(())
}
