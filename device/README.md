# Device workspace

This directory is the canonical Rust workspace for Focus Timer.

- `crates/focus-core` is the pure `no_std` domain model.
- `crates/focus-firmware` contains the ESP32-C3 runtime and hardware adapters.
- `crates/focus-protocol` will contain transport-independent synchronization
  models and codecs.

Run host checks from the repository root with `./scripts/check-device.sh` and
build the default ESP32 firmware with `./scripts/build-firmware.sh`.
