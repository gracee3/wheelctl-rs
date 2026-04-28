# wheelctl-rs

`wheelctl-rs` is a small Linux utility that turns a configured USB mouse or
rotary encoder exposed through evdev into a volume/control wheel.

Version 1 is deliberately narrow:

- grab one or more configured evdev devices exclusively
- suppress normal pointer movement, buttons, and other unmapped events while running
- map vertical scroll wheel events (`REL_WHEEL`) to PipeWire volume changes through `wpctl`
- run in the foreground as `wheelctl run`

Systemd support is intended to live in Justfile scaffolding later. The
`wheelctl` binary does not install services, daemonize, or fork.

## Platform

Linux only. The program reads `/dev/input/event*` devices through evdev.

## Requirements

- Rust toolchain for building from source
- Linux evdev input devices
- PipeWire/WirePlumber `wpctl` available on `PATH`
- permission to read the selected `/dev/input/event*` device

Many distributions restrict `/dev/input/event*` access to `root` or an `input`
group. Prefer a targeted udev rule or group-based access for the specific
device you want to dedicate as a wheel.

## Warning

When `grab = true`, the configured device is grabbed exclusively. While
`wheelctl run` is active, that mouse or encoder will stop acting like a normal
desktop pointer/button device. This is the intended behavior for a dedicated
control wheel.

Keep another keyboard or pointing device available while testing.

## Basic Flow

List readable input devices:

```sh
cargo run -- devices
```

Probe a candidate device:

```sh
cargo run -- probe /dev/input/by-id/usb-Dell_USB_Optical_Mouse-event-mouse
```

Add it to the config:

```sh
cargo run -- add /dev/input/by-id/usb-Dell_USB_Optical_Mouse-event-mouse
```

Review the parsed config:

```sh
cargo run -- show
```

Run in the foreground:

```sh
cargo run -- run
```

## Commands

```text
wheelctl devices
wheelctl probe <event-device-or-by-id-path>
wheelctl add <event-device-or-by-id-path>
wheelctl remove <device-key>
wheelctl show
wheelctl run
```

## Config

The config file is read from:

```text
~/.config/wheelctl/config.toml
```

Example:

```toml
[[devices]]
key = "dell-usb-optical-mouse"
name = "Dell USB Optical Mouse"
path = "/dev/input/by-id/usb-Dell_USB_Optical_Mouse-event-mouse"
phys = "usb-0000:00:14.0-1/input0"
vendor_id = "413c"
product_id = "301a"
enabled = true
grab = true

[devices.mappings.scroll_vertical]
enabled = true
backend = "pipewire"
target = "volume"
step = "5%"

[devices.disabled]
movement = true
buttons = true
horizontal_scroll = true
```

`wheelctl add` derives `key` from the device name and writes a default stanza.
If you have multiple identical devices, edit the key to keep it unique and
human-readable.

## Justfile

Convenience commands:

```sh
just build
just check
just fmt
just test
just run
just devices
just install-systemd-user-placeholder
```

The systemd command is only a placeholder for future Justfile-only scaffolding.

## Limitations

- Only `REL_WHEEL` is mapped in v1.
- PipeWire volume is implemented by shelling out to `wpctl`.
- ALSA and PulseAudio volume control are not implemented.
- No udev rule generation or permission setup is provided.
- No systemd install support is built into the Rust binary.
- Device hotplug/reconnect handling is minimal; restart `wheelctl run` after
  reconnecting a configured device.
