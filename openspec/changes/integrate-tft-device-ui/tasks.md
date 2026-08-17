## 1. Pure Presentation Contract

- [x] 1.1 Introduce a bounded TFT view model derived only from `AppSnapshot`, including state label, preset name, formatted duration, progress, hint, and generation metadata.
- [x] 1.2 Add the physical catalog-confirmation TFT view without changing confirmation ownership or encoder semantics.
- [x] 1.3 Define pure cadence and fallback decisions for recent interaction, Running, quiescent states, delayed frames, and static degradation.
- [x] 1.4 Add host tests for all four session states, catalog confirmation, zero/full/clamped progress, long preset names, visible-second changes, and cadence/fallback selection.

## 2. Reusable ST7789 Adapter

- [x] 2.1 Extract ST7789 initialization, address-window commands, SPI mode/frequency, DMA sizing, and row-block transfer from the diagnostic into a production-safe display module.
- [x] 2.2 Replace production-path panics with typed initialization and transfer errors while retaining strict assertions only in the focused diagnostic.
- [x] 2.3 Extract the seamless procedural texture, RGB565 palette, interpolation, and foreground composition behind bounded renderer state.
- [x] 2.4 Render Idle, Running, Paused, Completed, and catalog-confirmation TFT views with readable preset, time, progress, state treatment, and interaction hint.
- [x] 2.5 Keep `tft-diagnostic` independently buildable through the shared adapter and preserve its current wiring presentation and animation smoke test.
- [x] 2.6 Add host-testable renderer tests for palette bounds, seam-free wrapped sampling, foreground priority, progress endpoints, and static fallback output.

## 3. Latest-Value Display Worker

- [x] 3.1 Implement a bounded latest-value render slot with monotonically ordered generations and non-blocking publisher behavior.
- [x] 3.2 Add host concurrency tests proving newer views replace pending older views, stale generations cannot win, and temporary contention is retried.
- [x] 3.3 Add a lower-priority display worker that owns TFT peripherals, animation phase, and row buffers while yielding between bounded transfers.
- [x] 3.4 Make state/selection updates bypass the normal animation delay and keep routine animation at or below the configured cadence.
- [x] 3.5 Add bounded display health, log throttling, retry/backoff, and headless/static fallback behavior for initialization and transfer failures.
- [x] 3.6 Prove through adapter tests or diagnostics that a blocked/failing display worker cannot block authoritative session, input, persistence, journal, buzzer, or BLE work.

## 4. SuperMini Production Runtime

- [x] 4.1 Add and document the proposed SuperMini map: TFT GPIO6/7/3/4/10, encoder GPIO0/20/5, buzzer GPIO1, 3.3 V TFT power/backlight, and common ground.
- [ ] 4.2 Move encoder S2 from GPIO4 to GPIO20 in the selected production profile and preserve clockwise/right semantic mapping after physical validation.
- [x] 4.3 Replace synchronous OLED initialization/rendering in the production profile with TFT worker startup and non-blocking view publication.
- [x] 4.4 Publish the newest snapshot on boot, selection changes, state transitions, visible-second changes, and catalog-confirmation changes without adding a second state owner.
- [x] 4.5 Preserve OLED and other focused diagnostics as alternative firmware features and keep the production/diagnostic mutual-exclusion guard correct.
- [ ] 4.6 Flash a static-real-snapshot checkpoint and verify boot, Idle selection, Running countdown, Paused, Completed, and catalog-confirmation screens before enabling animation.

## 5. Animated Combined Runtime

- [x] 5.1 Enable bounded navy/turquoise procedural animation behind the real TFT views with foreground contrast limits and no visible tile seams.
- [ ] 5.2 Verify accepted input and state changes appear within 250 milliseconds while animation is active; tune worker priority, block size, and cadence if required.
- [x] 5.3 Verify delayed frames coalesce safely and switch to static rendering without changing countdown accuracy or replaying stale UI.
- [ ] 5.4 Confirm display initialization/transfer fault injection produces no panic/reboot loop and leaves encoder, timer, buzzer, storage, journal, and BLE behavior available.

## 6. Automated and Build Gates

- [x] 6.1 Run formatting, lint, host unit/integration tests, and `git diff --check` for all affected workspace crates.
- [x] 6.2 Build production firmware plus TFT, OLED, encoder, buzzer, acceptance, persistence, journal, BLE, and radio-failure diagnostic feature combinations affected by the change.
- [x] 6.3 Record pre/post-TFT firmware size and static allocation deltas and reject a full-screen framebuffer or other allocation that removes required BLE/journal margin.
- [x] 6.4 Run strict OpenSpec validation and reconcile implementation evidence with this change's requirements and task checkboxes.

## 7. Breadboard Acceptance and Documentation

- [ ] 7.1 Validate boot/flash/monitor and encoder phase behavior with GPIO20 across reset, USB reconnect, held encoder levels, and normal rotation.
- [ ] 7.2 Exercise standalone selection, start, pause, resume, cancel, completion, settings restore, custom presets, and journal recording while observing TFT and buzzer agreement.
- [ ] 7.3 Repeat lifecycle and rapid encoder tests while the unlocked iPhone is connected, live status is active, and session/catalog synchronization is occurring.
- [ ] 7.4 Record animation/static cadence, input latency, watchdog/reset behavior, idle/connected/synchronizing free heap, display-worker stack reserve, and whole-device USB current where the meter permits.
- [ ] 7.5 Verify unplugged/replugged TFT and induced transfer failures recover or remain safely headless without corrupting settings, presets, or journal data.
- [x] 7.6 Update hardware and developer documentation with exact module markings, GPIO/wiring table, power boundary, build/flash/monitor commands, diagnostic recovery, measured evidence, and explicit battery/OTA/enclosure deferral.
- [ ] 7.7 Restore and flash the combined production firmware, confirm no destructive diagnostic altered valid NVS data, and mark the breadboard profile ready for soldered-prototype-board assembly.
