# wheelctl-rs

Linux-only evdev utility that grabs a configured mouse/encoder and maps
`REL_WHEEL` to PipeWire volume through `wpctl`.

## Cargo Commands

```sh
cargo run -- devices
cargo run -- probe /dev/input/by-id/usb-PixArt_Dell_MS116_USB_Optical_Mouse-event-mouse
cargo run -- events /dev/input/by-id/usb-PixArt_Dell_MS116_USB_Optical_Mouse-event-mouse
cargo run -- add /dev/input/by-id/usb-PixArt_Dell_MS116_USB_Optical_Mouse-event-mouse
cargo run -- show
cargo run -- osd-test
cargo run -- run
```

`events` grabs the device by default and hides `REL_X`/`REL_Y` movement:

```sh
cargo run -- events /dev/input/by-id/usb-PixArt_Dell_MS116_USB_Optical_Mouse-event-mouse
cargo run -- events --movement /dev/input/by-id/usb-PixArt_Dell_MS116_USB_Optical_Mouse-event-mouse
cargo run -- events --no-grab /dev/input/by-id/usb-PixArt_Dell_MS116_USB_Optical_Mouse-event-mouse
```

## Config

Config path:

```text
~/.config/wheelctl/config.toml
```

Full example. Fine mode is the startup/default mode (`fine_step = "1%"`);
middle click toggles to normal step mode (`step = "5%"`) and back.

```toml
[[devices]]
key = "pixart-dell-ms116-usb-optical-mouse"
name = "PixArt Dell MS116 USB Optical Mouse"
path = "/dev/input/by-id/usb-PixArt_Dell_MS116_USB_Optical_Mouse-event-mouse"
phys = "usb-0000:00:14.0-2/input0"
vendor_id = "413c"
product_id = "301a"
enabled = true
grab = true

[devices.mappings.scroll_vertical]
enabled = true
backend = "pipewire"
target = "volume"
step = "5%"
fine_step = "1%"

[devices.mappings.mode_button]
enabled = true
button = "BTN_MIDDLE"
behavior = "toggle"

[devices.disabled]
movement = true
buttons = true
horizontal_scroll = true

[osd]
enabled = true
backend = "libnotify"
```

The OSD readout shows the current volume plus `normal` or `fine`.

## OSD

OSD is implemented separately in `src/osd.rs` by shelling out to
`notify-send`. wheelctl sends low-urgency, transient, short-lived libnotify
notifications and replaces the previous notification when the daemon supports
replacement IDs.

Screen position, fade timing, and visual style are controlled by your
notification daemon, not by `notify-send` itself. On i3, use something like
`dunst`; for bottom-right placement, merge
`packaging/dunst/wheelctl-bottom-right.conf` into `~/.config/dunst/dunstrc`:

```ini
[global]
origin = bottom-right
offset = (12, 48)
icon_position = off

[wheelctl]
appname = wheelctl
urgency = low
timeout = 1
format = "<b>%s</b>"
```

Then test:

```sh
cargo run -- osd-test
```

## Justfile

```sh
just build
just check
just fmt
just test
just run
just devices
just install
just install-systemd-user
just uninstall-systemd-user
```

## systemd User Service

wheelctl only supports a systemd user service. Do not install it as a system
service: it needs the user's PipeWire session, notification daemon, and config
directory. The Rust binary does not install or manage systemd; the Justfile
copies the provided user unit from `packaging/systemd/user/wheelctl.service`.

Manual commands:

```sh
cargo install --path .
install -Dm644 packaging/systemd/user/wheelctl.service ~/.config/systemd/user/wheelctl.service
systemctl --user daemon-reload
systemctl --user enable --now wheelctl.service
systemctl --user status wheelctl.service
```

Uninstall the user service:

```sh
systemctl --user disable --now wheelctl.service
rm -f ~/.config/systemd/user/wheelctl.service
systemctl --user daemon-reload
```

## Notes

- Requires Linux evdev device access, usually via the `input` group or a udev
  rule.
- Requires PipeWire/WirePlumber `wpctl`.
- When `grab = true`, the configured mouse stops acting like a normal desktop
  mouse while `wheelctl run` is active.
- The systemd user service runs at low CPU and I/O priority. Runtime power use
  is otherwise tiny: wheelctl blocks waiting for evdev events and shells out to
  `wpctl` only on wheel changes.
- Optical mouse sensor LEDs usually are not exposed through evdev or the kernel
  LED interface. wheelctl cannot turn off the Dell MS116 optical LED unless the
  device exposes a separate driver-specific control.
- PulseAudio, ALSA control, hotplug handling, and native PipeWire APIs are out
  of scope for v1.
