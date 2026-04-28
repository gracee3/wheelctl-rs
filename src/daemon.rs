use crate::backend::{BackendKind, change_volume, current_volume, ensure_available};
use crate::config::{
    ButtonCode, Config, DeviceConfig, ModeButtonBehavior, ModeButtonMapping, ScrollVerticalMapping,
};
use crate::device::{is_vertical_wheel, open_evdev_device};
use crate::osd::{Notifier, OsdConfig};
use anyhow::{Context, Result};
use evdev::{EventSummary, KeyCode};
use std::thread;
use tracing::{error, info, warn};

pub fn run(config: Config) -> Result<()> {
    let osd_config = config.osd;
    let enabled_devices: Vec<DeviceConfig> = config
        .devices
        .into_iter()
        .filter(|device| device.enabled)
        .collect();

    if enabled_devices.is_empty() {
        warn!("no enabled devices configured");
        return Ok(());
    }

    ensure_backends_available(&enabled_devices)?;

    let mut handles = Vec::new();
    for device_config in enabled_devices {
        let name = device_config.name.clone();
        let osd_config = osd_config.clone();
        handles.push(thread::spawn(move || {
            if let Err(error) = run_device(device_config, osd_config) {
                error!(device = %name, error = ?error, "device worker stopped");
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

fn ensure_backends_available(devices: &[DeviceConfig]) -> Result<()> {
    let needs_pipewire = devices.iter().any(|device| {
        let mapping = &device.mappings.scroll_vertical;
        mapping.enabled && mapping.backend == BackendKind::Pipewire && mapping.target == "volume"
    });

    if needs_pipewire {
        ensure_available(BackendKind::Pipewire)?;
    }

    Ok(())
}

fn run_device(config: DeviceConfig, osd_config: OsdConfig) -> Result<()> {
    let mut device = open_evdev_device(&config.path)
        .with_context(|| format!("failed to open configured device {}", config.path))?;
    let mut mode_state = ModeState::default();
    let mut notifier = Notifier::new(osd_config);

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
                handle_vertical_scroll(&config, &mut notifier, &mode_state, event.value())?;
                continue;
            }

            if let EventSummary::Key(_, key, value) = event.destructure() {
                handle_mode_button(&config.mappings.mode_button, &mut mode_state, key, value);
                if mode_state.changed {
                    let label = if mode_state.active {
                        format!("fine ({})", config.mappings.scroll_vertical.fine_step)
                    } else {
                        format!("normal ({})", config.mappings.scroll_vertical.step)
                    };
                    notifier.show("Wheel mode", &label);
                    mode_state.changed = false;
                }
            }

            // Reading from a grabbed device and intentionally doing nothing here
            // suppresses pointer movement, buttons, and other unmapped events.
        }
    }
}

fn handle_vertical_scroll(
    config: &DeviceConfig,
    notifier: &mut Notifier,
    mode_state: &ModeState,
    value: i32,
) -> Result<()> {
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
    let step = active_step(mapping, mode_state);
    for _ in 0..value.unsigned_abs() {
        change_volume(mapping.backend, step, increase)?;
    }

    match current_volume(mapping.backend) {
        Ok(volume) => {
            let volume = normalize_volume_display(&volume);
            notifier.show("Volume", &volume);
        }
        Err(error) => {
            warn!(device = %config.name, error = %error, "failed to read volume for OSD")
        }
    }

    Ok(())
}

fn handle_mode_button(
    mapping: &ModeButtonMapping,
    state: &mut ModeState,
    key: KeyCode,
    value: i32,
) {
    if !mapping.enabled || !button_matches(mapping.button, key) {
        return;
    }

    match mapping.behavior {
        ModeButtonBehavior::Toggle if value == 1 => {
            state.active = !state.active;
            state.changed = true;
        }
        ModeButtonBehavior::Hold => {
            let active = value != 0;
            if state.active != active {
                state.active = active;
                state.changed = true;
            }
        }
        _ => {}
    }
}

fn active_step<'a>(mapping: &'a ScrollVerticalMapping, state: &ModeState) -> &'a str {
    if state.active {
        &mapping.fine_step
    } else {
        &mapping.step
    }
}

fn button_matches(configured: ButtonCode, key: KeyCode) -> bool {
    match configured {
        ButtonCode::BtnRight => key == KeyCode::BTN_RIGHT,
    }
}

fn normalize_volume_display(raw: &str) -> String {
    let muted = raw.to_ascii_lowercase().contains("muted");
    let Some(value) = raw
        .split_whitespace()
        .find_map(|part| part.parse::<f32>().ok())
    else {
        return raw.to_string();
    };

    let percent = (value * 100.0).round() as i32;
    if muted {
        format!("{percent}% muted")
    } else {
        format!("{percent}%")
    }
}

#[derive(Debug, Default)]
struct ModeState {
    active: bool,
    changed: bool,
}

#[cfg(test)]
mod tests {
    use super::{ModeState, active_step, handle_mode_button, normalize_volume_display};
    use crate::config::{ButtonCode, ModeButtonBehavior, ModeButtonMapping, ScrollVerticalMapping};
    use evdev::KeyCode;

    #[test]
    fn right_button_toggle_switches_fine_mode() {
        let mapping = ModeButtonMapping {
            enabled: true,
            button: ButtonCode::BtnRight,
            behavior: ModeButtonBehavior::Toggle,
        };
        let mut state = ModeState::default();

        handle_mode_button(&mapping, &mut state, KeyCode::BTN_RIGHT, 1);
        assert!(state.active);
        assert!(state.changed);

        state.changed = false;
        handle_mode_button(&mapping, &mut state, KeyCode::BTN_RIGHT, 0);
        assert!(state.active);
        assert!(!state.changed);

        handle_mode_button(&mapping, &mut state, KeyCode::BTN_RIGHT, 1);
        assert!(!state.active);
    }

    #[test]
    fn hold_mode_tracks_button_state() {
        let mapping = ModeButtonMapping {
            enabled: true,
            button: ButtonCode::BtnRight,
            behavior: ModeButtonBehavior::Hold,
        };
        let mut state = ModeState::default();

        handle_mode_button(&mapping, &mut state, KeyCode::BTN_RIGHT, 1);
        assert!(state.active);

        handle_mode_button(&mapping, &mut state, KeyCode::BTN_RIGHT, 0);
        assert!(!state.active);
    }

    #[test]
    fn active_step_uses_fine_step_when_mode_is_active() {
        let mapping = ScrollVerticalMapping::enabled_pipewire_volume();
        let mut state = ModeState::default();
        assert_eq!(active_step(&mapping, &state), "5%");

        state.active = true;
        assert_eq!(active_step(&mapping, &state), "1.5%");
    }

    #[test]
    fn volume_display_is_normalized_for_osd() {
        assert_eq!(normalize_volume_display("Volume: 0.90"), "90%");
        assert_eq!(
            normalize_volume_display("Volume: 0.34 [MUTED]"),
            "34% muted"
        );
        assert_eq!(normalize_volume_display("unexpected"), "unexpected");
    }
}
