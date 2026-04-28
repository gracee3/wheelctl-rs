build:
    cargo build

check:
    cargo check

fmt:
    cargo fmt

test:
    cargo test

run:
    cargo run -- run

devices:
    cargo run -- devices

install:
    cargo install --path .

install-systemd-user:
    install -Dm644 packaging/systemd/user/wheelctl.service ~/.config/systemd/user/wheelctl.service
    systemctl --user daemon-reload
    systemctl --user enable --now wheelctl.service

uninstall-systemd-user:
    -systemctl --user disable --now wheelctl.service
    rm -f ~/.config/systemd/user/wheelctl.service
    systemctl --user daemon-reload
