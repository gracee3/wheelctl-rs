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
- optional: `notify-send` for libnotify OSD notifications

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
wheelctl events <event-device-or-by-id-path>
wheelctl add <event-device-or-by-id-path>
wheelctl remove <device-key>
wheelctl show
wheelctl osd-test
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
fine_step = "1.5%"

[devices.mappings.mode_button]
enabled = true
button = "BTN_RIGHT"
behavior = "toggle"

[devices.disabled]
movement = true
buttons = true
horizontal_scroll = true

[osd]
enabled = true
backend = "libnotify"
```

`wheelctl add` derives `key` from the device name and writes a default stanza.
If you have multiple identical devices, edit the key to keep it unique and
human-readable.

`mode_button` is consumed by the grabbed device. With the default toggle
behavior, right click switches between the normal `step` and `fine_step`.
Set `behavior = "hold"` if you prefer fine mode only while the button is held.
The active mode is shown when it changes and included in volume OSD updates.

`osd` is optional and separate from volume control. When enabled with the
libnotify backend, `wheelctl` shells out to `notify-send` after volume or mode
changes. Missing or failing notifications are logged and do not stop the input
loop. Run `wheelctl osd-test` to verify your desktop notification daemon is
visible before relying on mode toggles.

On i3, `notify-send` usually needs a notification daemon such as `dunst`
running. If `wheelctl osd-test` times out or does not show anything, start a
daemon before relying on OSD state.

wheelctl keeps libnotify OSD updates low urgency, transient, short-lived, and
replaced in place when the notification daemon supports replacement IDs. Fade
timing and visual style are controlled by the notification daemon, for example
your `dunst` configuration.

`wheelctl events <path>` is a small diagnostic helper for confirming which
button and wheel event codes a device emits.

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
- OSD notifications depend on the desktop notification daemon and may look
  different across desktop environments.
- No udev rule generation or permission setup is provided.
- No systemd install support is built into the Rust binary.
- Device hotplug/reconnect handling is minimal; restart `wheelctl run` after
  reconnecting a configured device.
