use crate::config::{DeviceConfig, DisabledEvents, Mappings};
use anyhow::{Context, Result};
use evdev::{Device, EventType, RelativeAxisCode};
use std::fs;
use std::io;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub path: PathBuf,
    pub name: String,
    pub phys: Option<String>,
    pub unique: Option<String>,
    pub bus: String,
    pub vendor_id: String,
    pub product_id: String,
    pub version: String,
    pub event_types: Vec<String>,
    pub keys: Vec<String>,
    pub relative_axes: Vec<String>,
}

impl DeviceInfo {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let device = open_evdev_device(path)?;
        Ok(Self::from_device(path.to_path_buf(), &device))
    }

    fn from_device(path: PathBuf, device: &Device) -> Self {
        let input_id = device.input_id();
        Self {
            path,
            name: device.name().unwrap_or("unknown input device").to_string(),
            phys: device.physical_path().map(ToString::to_string),
            unique: device.unique_name().map(ToString::to_string),
            bus: input_id.bus_type().to_string(),
            vendor_id: format!("{:04x}", input_id.vendor()),
            product_id: format!("{:04x}", input_id.product()),
            version: format!("{:04x}", input_id.version()),
            event_types: device
                .supported_events()
                .iter()
                .map(|event| format!("{event:?}"))
                .collect(),
            keys: device
                .supported_keys()
                .map(|keys| keys.iter().map(|key| format!("{key:?}")).collect())
                .unwrap_or_default(),
            relative_axes: device
                .supported_relative_axes()
                .map(|axes| axes.iter().map(|axis| format!("{axis:?}")).collect())
                .unwrap_or_default(),
        }
    }

    pub fn is_candidate(&self) -> bool {
        self.relative_axes
            .iter()
            .any(|axis| axis == "REL_WHEEL" || axis == "REL_WHEEL_HI_RES")
    }

    pub fn to_default_config(&self) -> DeviceConfig {
        DeviceConfig {
            key: stable_key(&self.name),
            name: self.name.clone(),
            path: self.path.display().to_string(),
            phys: self.phys.clone(),
            vendor_id: Some(self.vendor_id.clone()),
            product_id: Some(self.product_id.clone()),
            enabled: true,
            grab: true,
            mappings: Mappings::default(),
            disabled: DisabledEvents::default(),
        }
    }
}

pub fn list_event_devices() -> Result<Vec<DeviceInfo>> {
    let mut devices = Vec::new();
    let entries = match fs::read_dir("/dev/input") {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(devices),
        Err(error) => return Err(error).context("failed to read /dev/input"),
    };

    for entry in entries {
        let entry = entry.context("failed to read /dev/input entry")?;
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if !file_name.starts_with("event") {
            continue;
        }

        match DeviceInfo::from_path(&path) {
            Ok(info) => devices.push(info),
            Err(error) => {
                tracing::warn!(path = %path.display(), error = %error, "skipping input device")
            }
        }
    }

    devices.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(devices)
}

pub fn print_devices(devices: &[DeviceInfo]) {
    if devices.is_empty() {
        println!("No /dev/input/event* devices found or readable.");
        println!("If devices exist but are unreadable, check permissions with:");
        println!("    ls -l /dev/input/event*");
        println!(
            "For a quick validation run, use sudo. For regular use, add your user to the input group and log out/in:"
        );
        println!("    sudo usermod -aG input $USER");
        return;
    }

    for device in devices {
        let marker = if device.is_candidate() { "*" } else { " " };
        println!(
            "{marker} {} - {} [bus={}, vendor={}, product={}]",
            device.path.display(),
            device.name,
            device.bus,
            device.vendor_id,
            device.product_id
        );
        if !device.relative_axes.is_empty() {
            println!("    relative: {}", device.relative_axes.join(", "));
        }
        if let Some(phys) = &device.phys {
            println!("    phys: {phys}");
        }
    }
}

pub fn print_probe(info: &DeviceInfo) {
    println!("path: {}", info.path.display());
    println!("name: {}", info.name);
    println!("phys: {}", info.phys.as_deref().unwrap_or("<none>"));
    println!("unique: {}", info.unique.as_deref().unwrap_or("<none>"));
    println!("bus: {}", info.bus);
    println!("vendor_id: {}", info.vendor_id);
    println!("product_id: {}", info.product_id);
    println!("version: {}", info.version);
    println!("event_types: {}", info.event_types.join(", "));
    println!("relative_axes: {}", display_list(&info.relative_axes));
    println!("keys/buttons: {}", display_list(&info.keys));
}

pub fn stable_key(name: &str) -> String {
    let mut key = String::new();
    let mut previous_dash = false;

    for ch in name.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            key.push(ch);
            previous_dash = false;
        } else if !previous_dash && !key.is_empty() {
            key.push('-');
            previous_dash = true;
        }
    }

    while key.ends_with('-') {
        key.pop();
    }

    if key.is_empty() {
        "input-device".to_string()
    } else {
        key
    }
}

pub fn is_vertical_wheel(event_type: EventType, code: u16) -> bool {
    event_type == EventType::RELATIVE && code == RelativeAxisCode::REL_WHEEL.0
}

pub fn open_evdev_device(path: impl AsRef<Path>) -> Result<Device> {
    let path = path.as_ref();
    Device::open(path).map_err(|error| open_error(path, error))
}

fn open_error(path: &Path, error: io::Error) -> anyhow::Error {
    if error.kind() != io::ErrorKind::PermissionDenied {
        return anyhow::Error::new(error).context(format!("failed to open {}", path.display()));
    }

    let resolved = fs::canonicalize(path).ok();
    let details = permission_details(resolved.as_deref().unwrap_or(path));
    let target = resolved
        .as_ref()
        .map(|resolved| format!(" -> {}", resolved.display()))
        .unwrap_or_default();

    anyhow::Error::new(error).context(format!(
        "permission denied opening {}{target}{details}\n\
         minimal workaround for validation: sudo cargo run -- <command>\n\
         regular user access: sudo usermod -aG input $USER, then log out and back in",
        path.display()
    ))
}

#[cfg(unix)]
fn permission_details(path: &Path) -> String {
    match fs::metadata(path) {
        Ok(metadata) => format!(
            " (mode {:04o}, uid {}, gid {})",
            metadata.mode() & 0o7777,
            metadata.uid(),
            metadata.gid()
        ),
        Err(_) => String::new(),
    }
}

#[cfg(not(unix))]
fn permission_details(_path: &Path) -> String {
    String::new()
}

fn display_list(items: &[String]) -> String {
    if items.is_empty() {
        "<none>".to_string()
    } else {
        items.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::stable_key;

    #[test]
    fn stable_key_slugifies_device_name() {
        assert_eq!(
            stable_key("Dell USB Optical Mouse"),
            "dell-usb-optical-mouse"
        );
        assert_eq!(stable_key("  Odd__Name!! "), "odd-name");
        assert_eq!(stable_key(""), "input-device");
    }
}
