## Purpose

Define the color-display behavior that makes the authoritative standalone timer state readable, responsive, and visually distinctive without coupling correctness to animation or TFT availability.

## ADDED Requirements

### Requirement: Authoritative state presentation
The TFT UI SHALL derive its content from the latest immutable device snapshot and SHALL NOT own or mutate preset, session, journal, or synchronization state. Idle, Running, Paused, and Completed MUST remain visually distinguishable and MUST show the selected or active preset name and remaining duration appropriate to that state.

#### Scenario: Idle presentation
- **WHEN** the authoritative device state is Idle
- **THEN** the TFT shows the selected preset name, its configured duration, and an affordance for starting it

#### Scenario: Running presentation
- **WHEN** the authoritative device state is Running
- **THEN** the TFT shows the active preset name, current remaining time, visible progress, and a running-state treatment

#### Scenario: Paused presentation
- **WHEN** the authoritative device state is Paused
- **THEN** the TFT shows the active preset name, frozen remaining time, retained progress, and an unambiguous paused treatment

#### Scenario: Completed presentation
- **WHEN** the authoritative device state is Completed
- **THEN** the TFT shows the active preset name, zero remaining time, completed progress, and an unambiguous completion treatment

#### Scenario: Physical catalog confirmation
- **WHEN** a phone-originated preset catalog update awaits physical approval
- **THEN** the TFT shows the proposed custom-preset count and clear encoder instructions for acceptance and rejection

### Requirement: Prompt visible interaction feedback
An accepted physical transition or preset-selection change SHALL be represented from the newest snapshot within 250 milliseconds while the display worker is healthy. Intermediate snapshots MAY be coalesced, but an older queued snapshot MUST NOT replace a newer one.

#### Scenario: Encoder changes selection
- **WHEN** an Idle encoder detent changes the selected preset
- **THEN** the new preset name and duration appear within 250 milliseconds without waiting for a background-animation cycle to finish

#### Scenario: Session transition
- **WHEN** a press starts, pauses, resumes, dismisses, or cancels a session
- **THEN** the next completed TFT frame represents the resulting authoritative state rather than the state preceding the input

#### Scenario: Countdown crosses a visible second
- **WHEN** Running remaining time crosses a displayed one-second boundary
- **THEN** the TFT advances to the corresponding remaining-time value without deriving elapsed time from rendered frame count

### Requirement: Best-effort atmospheric background
The TFT UI SHALL use the approved dark navy and turquoise visual direction and MAY render a subtle procedural background behind primary content. Animation SHALL be bounded best-effort work, MUST preserve foreground legibility, and MUST NOT be required for any timer interaction or state transition.

#### Scenario: Normal animated rendering
- **WHEN** the TFT worker and system resources are healthy
- **THEN** the background changes over time without visible tile seams or full-screen black flashes while primary text remains readable

#### Scenario: Animation is reduced or disabled
- **WHEN** animation is disabled, slowed, or cannot meet its render cadence
- **THEN** the newest state remains usable on a static branded background with the same primary information and controls

#### Scenario: Frame generation is delayed
- **WHEN** one frame takes longer than its target cadence
- **THEN** the device skips or coalesces visual work instead of changing monotonic time accounting or replaying stale frames

### Requirement: Display work is isolated from timer correctness
TFT initialization, frame generation, and SPI transfer MUST NOT block the authoritative input-and-session loop long enough to alter accepted encoder semantics, delay session completion, corrupt persistence or journal records, or break BLE request recovery. A TFT failure SHALL be logged and SHALL leave the standalone timer operational.

#### Scenario: TFT initialization fails
- **WHEN** the display cannot initialize at boot
- **THEN** the device continues to accept encoder input, run sessions, persist supported state, record session outcomes, produce buzzer feedback, and expose supported BLE behavior without a display

#### Scenario: TFT transfer fails during a session
- **WHEN** a frame transfer fails while a session is Running
- **THEN** the session continues from monotonic time, the failure is logged, and rendering later retries the newest snapshot or remains in headless mode

#### Scenario: Display worker is busy
- **WHEN** a new snapshot arrives while an older frame is still being produced
- **THEN** application processing continues and the renderer converges to the newest available snapshot without requiring every intermediate frame

### Requirement: Verified combined SuperMini hardware profile
The production hardware profile SHALL document and validate one non-conflicting ESP32-C3 SuperMini GPIO map for the ST7789V TFT, EC11 encoder, and active buzzer. The combined USB-powered prototype MUST preserve standalone operation with BLE disconnected and MUST remain usable while a phone is connected and receiving live status.

#### Scenario: Standalone combined-device exercise
- **WHEN** the assembled SuperMini prototype is operated without a phone through selection, start, pause, resume, cancel, and completion
- **THEN** encoder input, TFT state, buzzer feedback, settings, and journal outcomes agree with the authoritative session behavior

#### Scenario: Connected combined-device exercise
- **WHEN** the same lifecycle is exercised while the phone is connected and live status is active
- **THEN** TFT updates, BLE updates, encoder handling, completion, and journal outcomes converge without watchdog resets or lost authoritative transitions

#### Scenario: Production recovery after diagnostics
- **WHEN** the TFT diagnostic has been used for bring-up or fault isolation
- **THEN** a documented command restores the combined production firmware without erasing valid settings, presets, or journal data unless an explicit destructive diagnostic is selected
