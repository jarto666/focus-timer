## Context

See `proposal.md` for motivation and scope. The repository now contains host-tested core logic and working encoder/OLED firmware, while the product must combine that timing behavior with board-specific input, display, storage, and audible feedback. The MVP interaction hardware is an EC11 encoder, 0.96-inch 128x64 I2C SSD1306 OLED, and 3 V active buzzer, plus a breadboard, jumpers, and resistor assortment. The purchased 24-pixel 5 V WS2812 ring has exploratory diagnostic code but is intentionally disconnected and deferred because its reworked connections are unreliable and its diameter is unsuitable for the intended prototype. A photograph confirms that the delivered controller uses an `ESP32-C3-MINI-1` RISC-V module on a dual-USB-C development board with an onboard RGB LED; the exact board vendor/revision, flash size, USB-port roles, onboard-RGB GPIO, and complete pinout still require validation.

The four delta specs define the behavioral contract. The main design pressure is to prevent GPIO callbacks, display refreshes, flash writes, or future connectivity from becoming owners of session state or elapsed-time truth.

## Goals / Non-Goals

**Goals:**

- Make all preset and session transitions deterministic and unit-testable on the development host.
- Keep one authoritative application state owner and a monotonic source of elapsed-time truth.
- Make hardware operations replaceable adapters with explicit failure handling.
- Bring up the purchased OLED, encoder, and active buzzer with a reviewed power and logic-level plan.
- Reach useful breadboard checkpoints in the order toolchain, input, display, timer, persistence, then physical UX validation.
- Leave clear seams for later event export without creating a protocol or connectivity subsystem prematurely.

**Non-Goals:**

- Supporting multiple firmware runtimes or display/board combinations in the first prototype.
- Restoring a session after power loss or modeling wall-clock/calendar time.
- Building a general UI framework, runtime preset editor, settings menu, communication protocol, or background integration service.
- Making an enclosure, battery power budget, or production-manufacturing architecture part of this firmware design.
- Selecting or integrating an external addressable-LED ring, including its power, signal conditioning, mounting, control depth, and diffuser.

## Decisions

### 1. Use a two-crate workspace for the MVP

The workspace will contain:

```text
crates/
  focus-core/       pure domain and application behavior
  focus-firmware/   ESP32-C3 executable and hardware adapters
```

`focus-core` will contain preset validation, selection, session transitions, time calculations, application snapshots, and effects. It will use no ESP32-specific crates, perform no I/O, and remain compatible with `no_std`; host tests may enable `std` only as a test convenience.

`focus-firmware` will own startup, the event loop, logging, GPIO, encoder decoding, OLED, buzzer, monotonic clock, and non-volatile settings. Existing LED-ring diagnostics remain isolated experiments and are not part of the MVP runtime. A `focus-protocol` crate will not be created until a concrete desktop transport change defines a wire contract.

Alternative considered: one firmware crate with modules. It is initially simpler, but it makes it easy for domain code to acquire embedded dependencies and weakens host-test boundaries. A third protocol crate was also considered and rejected as speculative for this offline MVP.

### 2. Use `std + ESP-IDF` for the first firmware runtime

The firmware crate will target the delivered ESP32-C3 board through the Rust ESP-IDF ecosystem using the RISC-V `riscv32imc-esp-espidf` target. This prioritizes a short path to serial logging, timers, GPIO, and non-volatile storage for the prototype. Rust, ESP-IDF, and crate versions will be pinned together after a minimal build/flash spike succeeds on the actual board.

Alternative considered: `no_std + esp-hal`. It offers tighter control and a smaller runtime surface, but it moves more bring-up and peripheral integration risk into the first product milestone. Keeping `focus-core` `no_std`-compatible preserves a later migration path without requiring both firmware stacks now.

### 3. A single event loop owns application state

One firmware task will own the `App` value and process semantic events serially:

```text
raw GPIO ──> encoder adapter ──> InputEvent ──┐
monotonic timer ───────────────> TimeAdvanced ├─> App::handle(now, event)
boot/settings ─────────────────> Initialized ─┘          │
                                                        ├─> snapshot ─> OLED
                                                        └─> effects  ─> storage/buzzer/log
```

Interrupts or polling may collect raw input, but they will not mutate presets or sessions. Hardware adapters send events to the owner; the owner applies one transition and returns best-effort effects. Effects are executed after the new state is committed, so a failed beep, render, or settings save cannot roll it back.

Alternative considered: independent tasks sharing session state behind locks. That adds races between timer completion, pause input, and rendering without creating useful concurrency for this small device.

### 4. Make the state machine and input mapping explicit

The core transition table is:

| Current | Press | LongPress | Rotate | Time reaches zero |
|---|---|---|---|---|
| Idle | Start selected preset | No-op | Change selection | No-op |
| Running | Pause | Cancel to Idle | No-op | Complete once |
| Paused | Resume | Cancel to Idle | No-op | No-op |
| Completed | Return to Idle | No-op | No-op | No-op |

Unsupported events are safe no-ops. The active session snapshots the selected preset identifier, name, and duration so later selection or catalog changes cannot mutate it.

The encoder adapter initially uses polling at a stable cadence, a quadrature state decoder, switch debouncing, and mutually exclusive short/long press classification. Starting calibration values are a 25 ms switch debounce and an 800 ms long-press threshold; both are named firmware configuration values and must be tuned and documented during bench validation.

Alternative considered: GPIO-edge interrupts for the entire input pipeline. They can reduce polling work, but callback timing and bounce handling add complexity before encoder behavior is validated. The adapter boundary permits switching to interrupts later without changing core behavior.

### 5. Calculate remaining time from a monotonic deadline

On start or resume, Running stores a monotonic deadline derived from `now + remaining`. On each event or scheduled refresh, remaining time is `max(deadline - now, 0)`. Pausing snapshots that remaining duration and discards the active deadline. Crossing zero transitions to Completed and creates a completion effect once.

The firmware schedules refresh opportunities, but refresh count is not elapsed-time truth. This prevents drift when display I/O, logging, or another task delays a tick. Preset validation rejects durations that cannot be safely converted or added in the chosen monotonic representation.

Alternative considered: decrementing a counter once per second. It is visually simple but accumulates scheduler delay and complicates accurate pause boundaries.

### 6. Render immutable snapshots and play best-effort feedback, with no blocking UI transitions

After a state change and at visible second boundaries, the app exposes an immutable snapshot. The SSD1306 adapter formats that snapshot into one of four 128x64 screens. The OLED favors preset name and remaining time; state labels or icons make Paused and Completed unambiguous.

Rendering is retried from the newest snapshot after a transient failure. The active buzzer receives small on/off cadence patterns such as `Start` and `Complete`; it cannot be treated as a variable-frequency passive speaker. Buzzer playback is best effort and never participates in state transitions.

The existing WS2812 adapter and diagnostic binaries are retained only as technical evidence and a possible starting point for a later LED-ring change. The MVP event loop does not initialize, power, update, or depend on an external ring.

Alternative considered: screen objects that own navigation and timers. That couples correctness to a specific display library and makes headless tests less representative.

### 7. Persist a small versioned settings record

The initial settings payload contains a schema version and selected preset identifier only. On boot, the firmware loads and validates the record against the compiled catalog. Any read, decode, version, or identifier failure selects the compile-time default and logs the reason.

Selection changes update memory immediately. Persistence is coalesced after a short quiet period so rapidly rotating the encoder does not write flash for every detent. A failed write is logged and retried only after a later selection change or boot; it does not block use. Session state is deliberately absent from the record.

Alternative considered: persisting complete preset definitions and active sessions. Both introduce migrations and recovery semantics that the MVP does not need.

### 8. Use an electrically reviewed profile for the purchased modules

The initial hardware profile is:

- Controller: the photographed module is confirmed as `ESP32-C3-MINI-1`, so the firmware target is `riscv32imc-esp-espidf`. The carrier/development-board vendor and revision, flash size, roles of its two USB-C connectors, USB/serial implementation, complete pinout, and onboard-RGB GPIO must still be identified before the GPIO map and flashing workflow are finalized. The onboard RGB LED is available only as a bring-up diagnostic and is not part of the MVP interface.
- Power: USB is the default and only initial controller power input. The breadboard power module is optional, and neither it nor another external supply is connected in parallel with USB until its topology and backfeed behavior are understood.
- OLED: purchased 0.96-inch white 128x64 four-pin I2C SSD1306 module. Prefer 3.3 V module power so any onboard I2C pull-ups do not expose ESP32 GPIO to 5 V; confirm the actual pin order, supply range, pull-ups, and bus address on the delivered module.
- Encoder: purchased EC11 rotary encoder with push switch and knob. Encoder and switch signals use 3.3 V-domain pull-ups; inspect whether the delivered module already contains pull-ups before adding external ones.
- Buzzer: the standalone purchased 3 V 12 mm active buzzer is the preferred MVP device; the kit active buzzer is a substitute and the passive buzzer is reserved for a later tonal-feedback experiment. Verify polarity and operating current. Drive the selected active buzzer through a transistor and appropriate protection only when its measured/datasheet load is not safe for direct GPIO.
- LED ring: the purchased 24-pixel 5 V WS2812 ring is not connected in the MVP. Earlier low-brightness and full-ring diagnostics remain documented, but later hardware selection must begin a new review of supply current, VBUS distribution, local bulk/ceramic decoupling, data resistance, 3.3-to-5 V signal compatibility, mechanical diameter, and diffusion rather than inheriting the temporary breadboard circuit as a product design.

The starter kit additionally provides a breadboard power module, breadboard and jumper/Dupont wiring, pin headers, 22 pF and 100 nF ceramic capacitors, 10 uF and 100 uF electrolytic capacitors, resistors from 10 ohm through 1 Mohm, active/passive buzzers, buttons, discrete LEDs, one RGB LED, a potentiometer, `4N35`, and `74HC595`. The buttons and discrete LEDs are diagnostic spares; the passive buzzer is out of the MVP path. The `74HC595` is a shift register rather than a transparent one-wire buffer, and the `4N35` optocoupler is too slow for a WS2812 waveform, so neither should be treated as a future ring data-level solution.

The inventory does not prove that every safety/interface part is present. A transistor/driver or protective diode is acquired only if the selected buzzer load demonstrates the need. Any buffer, larger bulk capacitor, or other external-LED interface part belongs to the later ring change. Required parts are acquired before the affected load or unstable configuration is used.

Alternative considered: wiring every module directly from its marketplace description. This is rejected because listing names do not establish actual pull-ups, current draw, input thresholds, board variant, or safe GPIO loading.

### 9. Treat documentation and validation as build outputs

Before the firmware adapter milestone is accepted, the repository must name the exact board, OLED address/pinout, encoder wiring, buzzer driver, active MVP power budget, GPIO assignment, Rust toolchain, target, ESP-IDF version, crate versions, and build/flash/monitor commands. It must also record that the external LED ring is disconnected and deferred. Host tests cover catalog validation and every state transition, including time jumps and completion idempotence. Bench checks cover electrical and physical behavior that automated host tests cannot establish.

## Risks / Trade-offs

- [ESP-IDF Rust setup or a chosen peripheral driver is incompatible with the actual board] → Prove build, flash, serial logging, and one GPIO before adding application layers; pin the working toolchain as one unit.
- [Polling misses encoder transitions during slow display I/O] → Keep display work outside input sampling, measure missed/duplicate detents, and move raw capture behind the same adapter to interrupts only if the bench test fails.
- [Flash wear from browsing presets] → Coalesce selection saves and write only when the stable identifier actually changes.
- [Long press feels slow or triggers accidental cancellation] → Keep the threshold configurable, log classified gestures during validation, and revisit confirmation only if observed errors justify the extra step.
- [A display failure leaves a running session hard to understand] → Continue timing, report errors over serial, retry rendering, and fail the hardware acceptance gate until the display path is reliable.
- [The active buzzer overloads or injects noise into a GPIO] → Confirm its current and construction, use a transistor driver/protection as needed, and test the buzzer independently before event-loop integration.
- [`std + ESP-IDF` later constrains power use or binary size] → Keep core portable and reassess the firmware runtime in a separate change when battery or production constraints exist.

## Migration Plan

There is no deployed firmware or stored data to migrate. Delivery proceeds through reversible checkpoints:

1. Establish pinned toolchain and flash a serial-logging skeleton to the selected board.
2. Review the GPIO, USB power path, voltage, current, and interface circuit; classify the power module and acquire only the buzzer protection/drive parts proven necessary.
3. Add and verify the encoder, OLED, and buzzer adapters independently with diagnostic firmware behavior. Keep the external LED ring disconnected.
4. Integrate the already host-tested core through the single event loop.
5. Add the version-1 settings record; clearing the prototype's non-volatile namespace returns it to defaults.
6. Run the documented host and bench acceptance checks before declaring the change applied.

If a hardware integration must be rolled back, retain the last buildable checkpoint and the passing core tests; no external API or user data migration blocks reversal.

## Open Questions

- What carrier/development-board vendor and revision, flash size, two USB-C port roles, and USB/serial path accompany the confirmed `ESP32-C3-MINI-1` module?
- What pin order, I2C address, pull-ups, and supported supply range are present on the delivered SSD1306 module?
- What are the delivered buzzer's operating current and construction, and therefore which driver/protection circuit is required?
- What model/current/backfeed behavior does the kit breadboard power module have, and is there any reason to use it instead of USB for this prototype?
- Which GPIO drives the onboard RGB LED, and which complete GPIO allocation avoids boot-strapping pins, the onboard LED, USB/serial functions, and other board conflicts?

These choices must be recorded during the hardware-definition task, but they do not change the capability specs, crate boundary, state machine, explicit LED-ring deferral, or ordered milestone structure.
