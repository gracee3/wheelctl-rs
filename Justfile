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

# Placeholder for future user-level systemd scaffolding.
install-systemd-user-placeholder:
    @echo "systemd user install support is planned for a future Justfile-only step"
