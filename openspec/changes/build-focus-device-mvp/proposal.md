## Why

Starting a focus session should not require opening the same phone or computer that creates distraction. The first useful increment is a tactile, predictable, offline timer that turns a named preset into an active focus session within a few seconds and remains independent of every optional integration.

## What Changes

- Establish a Rust workspace for the delivered ESP32-C3-MINI-1-based firmware prototype, targeting `riscv32imc-esp-espidf`, with pure, host-testable domain logic separated from hardware adapters.
- Provide a compile-time list of named focus presets and encoder-first selection that can start the visible preset with one press.
- Implement deterministic session behavior for start, countdown, pause, resume, cancel, completion, and return to idle.
- Use the purchased EC11 rotary encoder, 0.96-inch 128x64 I2C SSD1306 OLED, and 3 V active buzzer as the MVP interaction hardware.
- Translate debounced encoder gestures into semantic input events and render idle, running, paused, and completed states on the OLED.
- Provide short start and completion feedback through the buzzer without making audible feedback part of timer correctness.
- Persist the selected preset and basic settings locally, while booting with safe defaults when storage is empty or unreadable.
- Document and validate the exact prototype board, display, GPIO map, wiring, toolchain, dependency versions, build, flash, test, and serial-monitor commands before hardware integration is considered complete.
- Validate the physical flow on a breadboard, including the five-second start target, encoder reliability, readability, long-press discoverability, and buzzer volume.
- Keep Spotify, desktop/mobile apps, networking, wireless provisioning, OTA, battery operation, analytics, external addressable-LED feedback, custom PCB work, and enclosure design outside this MVP change. Selection of a smaller, mechanically suitable LED ring and its power, signaling, mounting, and diffusion belong to a later version.

Assumptions requiring an early hardware decision or bench validation:

- The delivered controller is confirmed by photograph to use an `ESP32-C3-MINI-1` RISC-V module on a dual-USB-C development board with an onboard RGB LED. The board vendor/revision, flash size, roles of the two USB-C connectors, USB/serial path, onboard-RGB GPIO, and complete pinout still require on-device or documentation validation before the final GPIO map is pinned.
- The SSD1306 display, EC11 encoder, and active buzzer are known, but their actual module pinouts, current draw, pull-ups, logic levels, and safe drive circuits still require inspection or bench validation.
- USB is the default prototype power input. The kit's breadboard power module is optional and may be used only after its model/current path is identified; it is not a prerequisite for powering the controller.
- The purchased 24-pixel WS2812 ring was explored successfully and has reusable diagnostic/presentation code, but unreliable reworked connections and unsuitable physical size remove it from the MVP circuit and acceptance path. It remains disconnected until a later change selects a smaller, mechanically suitable module and reviews its 5 V power, 3.3-to-5 V signaling, decoupling, mounting, and diffusion.
- A short press starts, pauses, resumes, or dismisses completion according to the current state; a debounced long press cancels an active or paused session without a second confirmation in the first prototype.
- Active sessions are not restored after power loss; the device boots to idle with the last valid selected preset.

## Capabilities

### New Capabilities

- `preset-selection`: Browse a non-empty compile-time preset catalog, retain a valid selection, and start the visible preset quickly.
- `focus-session`: Run an offline, monotonic focus-session state machine with start, pause, resume, cancel, and completion semantics.
- `device-interface`: Convert the rotary encoder into semantic gestures and present state through the OLED and active buzzer.
- `device-settings`: Persist basic settings without making storage availability a prerequisite for booting or timing a session.

### Modified Capabilities

None. This is the first product change and there are no existing capability specifications.

## Impact

- Introduces a Rust workspace with a platform-independent core crate and an ESP32-C3 firmware crate targeting `riscv32imc-esp-espidf`; a protocol crate is deferred until a real desktop communication use case exists.
- Adds embedded HAL/runtime, SSD1306 display, EC11 encoder, storage, timing, logging, and test dependencies whose versions will be pinned after a minimal build/flash spike succeeds on the delivered board. Existing WS2812 diagnostic code is retained as an unintegrated experiment for a later version.
- Uses the purchased breadboard, power module, jumper/Dupont wires, pin headers, 100 nF/10 uF/100 uF capacitors, resistor assortment, active/passive buzzers, buttons, and indicator LEDs as available prototype inventory.
- Does not use the kit's `74HC595` shift register or `4N35` optocoupler as a level shifter. A buzzer transistor driver or related protection is acquired only when inspection and bench results justify it; LED-ring interface parts are deferred with the ring itself.
- Adds developer and hardware documentation plus both host-side automated tests and on-device validation procedures.
- Creates no network API, cloud dependency, account model, or desktop compatibility obligation.
