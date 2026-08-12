# Focus Timer

Offline focus-timer firmware for an ESP32-C3-MINI-1-based development board.
The active breadboard MVP uses an EC11 encoder and SSD1306 OLED; audible
feedback is the next adapter, while the external WS2812 ring is deferred to a
later hardware version.

The workspace separates pure, host-testable behavior from ESP32-specific
adapters:

- `crates/focus-core`: `no_std` preset, session, input, and settings behavior.
- `crates/focus-firmware`: `std + ESP-IDF` executable and hardware adapters.

See [docs/development.md](docs/development.md) for development commands and
[docs/hardware.md](docs/hardware.md) for the evolving prototype wiring record.
