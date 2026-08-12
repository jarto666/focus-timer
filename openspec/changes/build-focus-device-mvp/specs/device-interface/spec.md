## Purpose

Define the encoder-first interaction and restrained OLED and sound feedback that make session state understandable at desk distance.

## ADDED Requirements

### Requirement: Semantic encoder input
The device SHALL translate raw encoder signals into RotateLeft, RotateRight, Press, and LongPress events before they reach session or preset behavior. Debouncing SHALL prevent one physical action from producing unintended duplicate semantic events, and one button hold MUST NOT produce both Press and LongPress.

#### Scenario: One encoder detent
- **WHEN** the user rotates the encoder through one stable detent
- **THEN** exactly one rotation event in the detected direction is delivered to application behavior

#### Scenario: Short press
- **WHEN** the user presses and releases before the documented long-press threshold
- **THEN** exactly one Press event is delivered and no LongPress event is delivered

#### Scenario: Long press
- **WHEN** the user holds the encoder through the documented long-press threshold and releases it
- **THEN** exactly one LongPress event is delivered and no Press event is delivered

#### Scenario: Contact bounce
- **WHEN** the physical encoder or switch produces bouncing transitions within its debounce window
- **THEN** the transitions are coalesced into at most the single semantic action the user performed

### Requirement: State-specific display
The display SHALL render the current application state without owning or mutating that state. Idle, Running, Paused, and Completed MUST be visually distinguishable.

#### Scenario: Idle screen
- **WHEN** the device is Idle
- **THEN** the display shows the selected preset name and configured duration

#### Scenario: Running screen
- **WHEN** a session is Running
- **THEN** the display shows the active preset name, remaining time, and a running-state indication

#### Scenario: Paused screen
- **WHEN** a session is Paused
- **THEN** the display shows the active preset name, frozen remaining time, and an obvious paused indication

#### Scenario: Completed screen
- **WHEN** a session is Completed
- **THEN** the display shows the active preset name and an obvious completed indication with zero remaining time

### Requirement: Immediate interaction feedback
Accepted input SHALL be reflected on the next UI update without a blocking animation. Starting a session SHALL produce brief feedback, and completion SHALL produce a distinct audible pattern.

#### Scenario: Session starts
- **WHEN** a valid short press starts the displayed preset
- **THEN** the next display update shows Running and the device requests one short start-feedback pattern

#### Scenario: Session completes
- **WHEN** the session emits its single completion event
- **THEN** the next display update shows Completed and the device requests one completion-feedback pattern distinguishable from start feedback

### Requirement: Feedback adapters do not own correctness
An OLED or buzzer operation MUST NOT delay monotonic time accounting or roll back an accepted session transition. Transient rendering failures SHALL be retried on a later refresh, and buzzer failures SHALL leave session behavior unchanged.

#### Scenario: Buzzer is unavailable
- **WHEN** start or completion feedback cannot be emitted by the buzzer
- **THEN** the corresponding session state transition still completes and the failure is reported through development logging

#### Scenario: Display update fails
- **WHEN** one display render operation fails during an active session
- **THEN** the timer continues and a later refresh attempts to render the current state again

### Requirement: Physical usability validation
The breadboard prototype SHALL be evaluated with the purchased encoder, OLED, and active buzzer before the MVP is accepted.

#### Scenario: Bench UX gate
- **WHEN** the assembled prototype is exercised from normal desk distance
- **THEN** recorded validation covers encoder reliability, five-second preset start, paused-state clarity, long-press discoverability, OLED readability, and completion volume
