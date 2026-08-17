## Why

The purchased 2.0-inch 240x320 ST7789V display and ESP32-C3 SuperMini have now passed an on-device animated bring-up, while the production firmware still targets the earlier SSD1306 prototype. The next useful increment is one standalone firmware build that preserves the proven timer, encoder, buzzer, persistence, journal, and BLE behavior while making the new color display the real device interface.

## What Changes

- Promote the validated ST7789V SPI bring-up into a production display adapter for the ESP32-C3 SuperMini hardware profile.
- Render Idle, Running, Paused, and Completed snapshots on the 240x320 color display with the preset name, remaining time, state, and progress visible at desk distance.
- Retain the dark navy and turquoise visual direction and a subtle procedural mist background, while ensuring animation never owns timekeeping, input handling, BLE state, or application truth.
- Coalesce display work and provide a static fallback so rendering load or a transient TFT failure cannot block encoder actions, session completion, persistence, journal updates, buzzer feedback, or synchronization.
- Keep the independently flashable TFT diagnostic as a bring-up and fault-isolation tool, with an explicit path back to production firmware.
- Validate the combined SuperMini prototype with TFT, encoder, buzzer, storage, journal, and BLE active together before soldered-prototype-board assembly.
- Treat the earlier OLED as a development fallback rather than part of the new production hardware profile.
- Keep battery operation, soft-power circuitry, OTA updates, enclosure design, LED-ring integration, and a custom PCB outside this change.

Assumptions requiring bench validation:

- The verified TFT wiring remains `SCLK=GPIO6`, `MOSI=GPIO7`, `RES=GPIO3`, `DC=GPIO4`, `CS=GPIO10`, with `VCC` and `BLK` at 3.3 V and a common ground.
- The remaining encoder, buzzer, and optional battery-sense assignments can be moved to non-conflicting SuperMini pins without using unsafe boot-strapping behavior.
- The procedural background can meet an acceptable visual cadence while leaving enough CPU, stack, heap, and scheduling margin for BLE and the authoritative timer runtime.
- USB remains the only accepted power source during this change; full-screen animation current and thermal behavior still require measurement before battery-runtime decisions.

## Capabilities

### New Capabilities

- `tft-device-ui`: Present the authoritative standalone timer state on the ST7789V color display, including bounded best-effort animation, prompt interaction feedback, static degradation, and combined-device validation.

### Modified Capabilities

None. Existing session, preset, device-input, persistence, journal, and BLE contracts remain unchanged; this change adds a color-display capability around their existing snapshots and effects.

## Impact

- Affects the ESP32-C3 firmware entry point, hardware profile, GPIO allocation, display adapter, render scheduling, diagnostics, developer documentation, and on-device acceptance procedures.
- Reuses the existing `focus-core`, protocol, synchronization, input, clock, buzzer, NVS, journal, and BLE boundaries instead of creating a second application state owner.
- Adds no cloud service, account requirement, network dependency, or phone dependency to the timer path.
- Establishes the verified SuperMini plus ST7789V combination as the target for the soldered prototype, but does not yet authorize battery wiring or permanent enclosure assembly.
