## Purpose

Defines a reproducible product monorepo in which device, mobile, and shared protocol work can evolve independently without losing the validated firmware checkpoint.

## ADDED Requirements

### Requirement: Canonical product layout
The repository SHALL expose separate canonical roots for device software, mobile applications, and reusable cross-application packages, and SHALL keep OpenSpec and product documentation discoverable from the repository root.

#### Scenario: Developer locates product areas
- **GIVEN** a fresh repository checkout
- **WHEN** a developer inspects the root documentation
- **THEN** the documented layout identifies the device workspace, the mobile application, reusable packages, OpenSpec changes, and product documentation

#### Scenario: New application can be added without moving device code again
- **GIVEN** the canonical product layout
- **WHEN** a later web or desktop application is introduced
- **THEN** it can be added beside the mobile application without changing the canonical device workspace path

### Requirement: Reproducible root workflows
The repository SHALL document and provide root-invocable workflows for device host checks, firmware build, mobile checks, protocol compatibility checks, and the combined non-hardware validation suite.

#### Scenario: Fresh checkout validation
- **GIVEN** a development machine with the documented pinned prerequisites
- **WHEN** the developer runs the documented non-hardware validation workflow from the repository root
- **THEN** device host tests, TypeScript checks, mobile tests, and protocol compatibility checks run without relying on an undisclosed working directory

#### Scenario: Firmware flashing remains reproducible
- **GIVEN** the reorganized repository and a connected supported controller
- **WHEN** the developer follows the root documentation for build, flash, and monitor
- **THEN** the same firmware target and diagnostic feature modes remain available from their new canonical paths

### Requirement: Behavior-preserving device migration
Moving the Rust workspace SHALL NOT alter the accepted offline timer behavior, GPIO allocation, persisted-selection format, diagnostic feature behavior, or standalone boot path.

#### Scenario: Host checkpoint after migration
- **GIVEN** the device workspace has moved under its canonical monorepo directory
- **WHEN** the existing host formatting, lint, and test suites run
- **THEN** all previously passing checks still pass without changing their behavioral assertions

#### Scenario: On-device smoke test after migration
- **GIVEN** the default firmware built from the new path is flashed to the existing controller
- **WHEN** the user selects, starts, pauses, resumes, cancels, completes, dismisses, and reboots the timer
- **THEN** its observable offline behavior matches the validated breadboard checkpoint

### Requirement: Independent toolchains
Unavailable mobile tooling SHALL NOT prevent device-only checks, and unavailable ESP tooling or hardware SHALL NOT prevent mock-backed mobile development and tests.

#### Scenario: Device-only developer
- **GIVEN** Rust and the documented ESP toolchain are installed but Node.js mobile tooling is absent
- **WHEN** the developer runs a device-scoped host check or firmware build
- **THEN** the command does not require installation of mobile dependencies

#### Scenario: Mobile-only development
- **GIVEN** Node.js, the documented package manager, and mobile prerequisites are installed but no ESP32 is connected
- **WHEN** the developer runs the mobile application with the mock device and executes mobile tests
- **THEN** those workflows complete without flashing or connecting physical hardware
