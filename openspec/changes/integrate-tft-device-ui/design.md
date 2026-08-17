## Context

See `proposal.md` for motivation. The current production runtime owns one `App`, polls an EC11, advances monotonic time, executes buzzer/storage/journal effects, serves BLE, and synchronously renders a buffered SSD1306 view. The new ESP32-C3 SuperMini and 240x320 ST7789V have separately passed a 26 MHz SPI mode-3 diagnostic with DMA row blocks, procedural navy/turquoise mist, foreground UI, no tile seams, and approximately 7.4 frames per second.

The diagnostic also exposed the central integration constraint: generating and transferring a full 153,600-byte RGB565 frame in the authoritative loop would make encoder sampling and BLE service depend on display latency. The combined runtime therefore needs a latest-state display boundary rather than a larger synchronous replacement for the OLED call.

## Goals / Non-Goals

**Goals:**

- Keep `App` and monotonic time as the only owners of session truth.
- Establish one documented SuperMini GPIO profile for TFT, encoder, and buzzer.
- Reuse the validated ST7789 initialization and row-buffered renderer without allocating a full-screen framebuffer.
- Render real application and catalog-confirmation snapshots with the validated visual direction.
- Isolate bounded animation and SPI work so stale or failed frames cannot stall the authoritative loop.
- Preserve diagnostics and measurable rollback points throughout migration.

**Non-Goals:**

- Supporting runtime hot-plugging or simultaneous OLED and TFT production output.
- Playing stored video, decoding compressed animation, or adding external RAM or flash.
- Redesigning session, preset, journal, protocol, or mobile application semantics.
- Implementing battery power, soft power-off, deep sleep, OTA, an enclosure, an LED ring, or a custom PCB.
- Treating diagnostic frame rate as a product correctness dependency.

## Decisions

### 1. Pin one SuperMini production profile and leave boot/USB pins alone

The initial combined profile is:

| Function | GPIO / rail |
|---|---:|
| TFT SCLK | GPIO6 |
| TFT MOSI/SDA | GPIO7 |
| TFT RES | GPIO3 |
| TFT DC | GPIO4 |
| TFT CS | GPIO10 |
| TFT VCC and BLK | 3.3 V |
| Encoder S1 | GPIO0 |
| Encoder S2 | GPIO20 |
| Encoder KEY | GPIO5 |
| Active buzzer | GPIO1 |
| Common return | GND |

GPIO4 moves from the old encoder profile to TFT DC, so encoder S2 moves to GPIO20. GPIO2, GPIO8, and GPIO9 remain unused because of ESP32-C3 strapping/boot concerns; GPIO18 and GPIO19 remain untouched for native USB. GPIO21 remains available for later measurement or power work. The encoder direction mapping will be revalidated after the S2 move rather than assumed from wire color.

Alternative considered: retain the old encoder pins and move TFT DC. That would invalidate the already verified TFT harness for no behavioral gain and make diagnostic-to-production comparison harder.

### 2. Keep presentation pure and add a TFT-specific view model

`presentation.rs` remains the snapshot-to-copy boundary. It will add a display-neutral semantic mapping where useful and a bounded `TftView` carrying state label, preset name, formatted remaining time, progress, hint, and catalog-confirmation content. The model contains no SPI handles, frame counter, or mutable application state. Existing OLED presentation tests remain useful and the OLED diagnostic remains independently buildable, but production selects the TFT adapter.

Alternative considered: let the TFT renderer inspect `App` or `ProtocolSession` directly. That creates a second state reader with timing and ownership assumptions and makes host testing dependent on embedded graphics.

### 3. Give the TFT a lower-priority latest-value worker

The authoritative loop publishes immutable render requests through a single latest-value slot. Publishing is non-blocking: a newer request replaces an older pending request, and a temporary lock/contention failure leaves `render_pending` set for a later loop pass. A dedicated lower-priority FreeRTOS-backed Rust thread owns SPI, DC, RES, the animation phase, and all render buffers.

The worker checks for a newer view before each frame and between DMA row blocks. It yields between bounded blocks so higher-priority input, time, BLE, buzzer, and persistence work can run. It may omit intermediate countdown or animation frames but never applies an older generation after observing a newer one. Display success is not required to commit an application transition.

Alternative considered: render synchronously in the existing loop. The diagnostic's roughly 135-millisecond full-frame cost is incompatible with one-millisecond encoder polling and makes display faults part of timer scheduling. A second complete application task is also rejected because it would duplicate state ownership.

### 4. Reuse a row-buffered procedural renderer, not a video asset or full framebuffer

The production adapter extracts the ST7789 command/window/transfer logic from the diagnostic. It keeps DMA-backed row blocks, a small seamless noise texture, a 256-entry RGB565 palette, and bilinear low-resolution interpolation. Foreground UI is composed while each row block is generated. This avoids a 153,600-byte framebuffer and leaves heap for BLE, catalogs, and the persistent journal.

The initial target is at most 6 animated frames per second in Running or during recent interaction, with a lower cadence in quiescent Idle, Paused, and Completed states. A state or selection change bypasses the normal animation wait. If measured combined-system margins are insufficient, the same renderer freezes the last background or uses a static gradient while continuing prompt foreground updates.

Alternative considered: store and decode a generated smoke video. Four megabytes of flash, decoding cost, RAM pressure, and looping artifacts make that a poor fit. A full framebuffer would simplify composition but consume most of the largest immediately available internal heap region before BLE and runtime allocations are considered.

### 5. Treat animation phase as disposable visual state

Animation phase exists only inside the display worker. Reboot, display retry, snapshot replacement, or a transition into static fallback may reset or advance it without persistence. Remaining time and progress always come from the supplied authoritative snapshot; frame count never decrements a timer.

Alternative considered: synchronize animation to the session clock. It creates an unnecessary correctness coupling and would make harmless skipped frames appear as timer defects.

### 6. Fail headless and retry deliberately

Production display paths return errors instead of using the diagnostic's `expect` calls. Initialization failure starts the rest of the runtime without a display. A transfer failure logs a bounded diagnostic, retains the newest view, backs off, and attempts reinitialization or a later static render according to a small display health state. Repeated faults must not produce a reboot loop or unbounded serial logging.

The independent `tft-diagnostic` feature remains the first isolation step. Its documented build and flash command is paired with the production restore command; neither path clears NVS unless a separate destructive diagnostic is explicitly selected.

Alternative considered: panic so hardware faults are obvious. The bring-up diagnostic may remain strict, but production panic/reboot behavior would violate the standalone timer contract and was already observed visually as black flashing during development.

### 7. Validate scheduling and memory before soldering

Host tests cover `AppSnapshot`/confirmation-to-`TftView` mapping, progress bounds, generation ordering, cadence selection, and static fallback decisions. Build gates cover production plus every diagnostic feature combination affected by the extraction.

On-device evidence records encoder detents and presses during continuous rendering, timer lifecycle, buzzer cadence, BLE live status and reconnect, journal outcomes, watchdog/reset logs, firmware size, free heap, and display-worker stack high-water behavior. The soldered board is not assembled until this combined profile passes on the breadboard.

## Risks / Trade-offs

- [The renderer starves encoder polling on the single-core ESP32-C3] → Run it below the authoritative task, yield between row blocks, cap cadence, and fall back to static rendering if stress evidence shows missed input.
- [GPIO20 interacts with serial tooling on this board revision] → Validate boot, flash, monitor, encoder direction, and held-level behavior before making the harness permanent; move only the encoder phase if the test fails.
- [Continuous TFT work shortens later battery runtime] → Keep cadence policy explicit and measure current before the battery change; battery acceptance is not inferred from USB success.
- [A large display-worker stack reduces BLE or journal margin] → Keep row buffers bounded, measure stack high-water/free heap in idle, connected, and synchronizing states, and avoid a full framebuffer.
- [Procedural contrast obscures foreground text] → Clamp background luminance through the palette and validate every state at desk distance; static fallback uses the same contrast limits.
- [Older OLED-specific names leak through runtime code] → Migrate presentation and rendering boundaries incrementally while preserving the OLED diagnostic until TFT production acceptance passes.

## Migration Plan

1. Preserve the already-pushed TFT diagnostic checkpoint and document its verified wiring and measured cadence.
2. Add pure TFT view/cadence models and host tests without changing production hardware startup.
3. Extract a reusable ST7789 transport/renderer from the diagnostic and keep the diagnostic buildable.
4. Add the SuperMini GPIO profile and display worker behind a compile-time production selection; flash a static real-snapshot checkpoint first.
5. Enable bounded procedural animation and verify physical interaction before enabling BLE stress.
6. Run the combined standalone and connected acceptance matrix, record resource/current evidence, and restore the production feature set.
7. Update hardware/developer documentation and designate the profile ready for soldered-prototype-board assembly.

Rollback is a flash of commit `2c4e2dc` for isolated TFT bring-up or the last pre-TFT production firmware for the OLED-based runtime. Settings, custom presets, and journal schemas are unchanged by this change.

## Open Questions

- What exact foreground font sizes and animation cadences feel best at normal desk distance after the first real-state render?
- Does the delivered SuperMini expose GPIO20 reliably with the current USB/serial path under all boot conditions?
- What whole-device USB current is measured for static Idle, animated Idle, Running, and connected synchronization modes?
