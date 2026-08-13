# Prototype hardware record

For rebuilding the breadboard after controller-header repair, use the dedicated
[full wiring diagram and reconstruction checklist](wiring.md).

## Controller

- [Front photograph](controller-front.jpg): module marking `ESP32-C3-MINI-1`.
- Connected `espflash board-info`: ESP32-C3 revision v0.4, 40 MHz crystal,
  4 MB flash, Wi-Fi + BLE, MAC `08:92:72:9d:24:b0`.
- Carrier board: unbranded dual-USB-C `ESP32-C3-DevKitM-1`-compatible board;
  no vendor or carrier revision is present on the photographed silkscreen.
- `COM` connector: WCH USB-to-UART (`VID:PID 1A86:7522`). macOS detected
  the USB device but exposed no serial node without an additional WCH driver.
- `USB` connector: native Espressif USB JTAG/Serial (`VID:PID 303A:1001`),
  observed as `/dev/cu.usbmodem101`; this is the primary development path.
- Native USB uses GPIO18 (`USB_D-`) and GPIO19 (`USB_D+`).
- Onboard addressable RGB data uses GPIO8. It is a diagnostic only; GPIO8 is a
  strapping pin and is not allocated to an external peripheral.
- Reviewed provisional GPIO allocation is recorded below. The buzzer output stays
  conditional on its measured current and driver decision.

The carrier matches the DevKitM-1 header layout and adds a second USB-C path.
Header order below is top-to-bottom when the antenna is at the top and USB-C
connectors are at the bottom, matching the photographed silkscreen.

| Left header | Function | Right header | Function |
| --- | --- | --- | --- |
| GND | ground | GND | ground |
| 3V3 | 3.3 V | TX / GPIO21 | UART0 TX |
| 3V3 | 3.3 V | RX / GPIO20 | UART0 RX |
| GPIO2 | ADC1_CH2, strapping | GND | ground |
| GPIO3 | ADC1_CH3 | GPIO9 | BOOT, strapping |
| GND | ground | GPIO8 | onboard RGB, strapping |
| RST | CHIP_PU | GND | ground |
| GND | ground | GPIO7 | general I/O |
| GPIO0 | ADC1_CH0 | GPIO6 | general I/O |
| GPIO1 | ADC1_CH1 | GPIO5 | ADC2_CH0 |
| GPIO10 | general I/O | GPIO4 | ADC1_CH4 |
| GND | ground | GND | ground |
| 5V out | carrier-labelled 5 V rail | GPIO18 | native USB D- |
| 5V in | carrier-labelled 5 V rail | GPIO19 | native USB D+ |
| GND | ground | GND | ground |

Reference baseline: Espressif
[`ESP32-C3-DevKitM-1` hardware guide](https://docs.espressif.com/projects/esp-idf/en/latest/esp32c3/hw-reference/esp32c3/user-guide-devkitm-1.html)
and [`ESP32-C3-MINI-1` datasheet](https://www.espressif.com/documentation/esp32-c3-mini-1_datasheet_en.pdf).

Verified read-only probe:

```text
espflash board-info --port /dev/cu.usbmodem101 --before usb-reset --non-interactive
Chip type:         esp32c3 (revision v0.4)
Crystal frequency: 40 MHz
Flash size:        4MB
Features:          WiFi, BLE
```

## Firmware bring-up evidence

Validated on 2026-08-08 through the native `USB` connector at
`/dev/cu.usbmodem101`:

```text
App/part. size:    570,576/4,128,768 bytes, 13.82%
I (...) app_init: Project name:     libespidf
I (...) app_init: ESP-IDF:          v5.5.3
I (...) focus_firmware: focus-firmware 0.1.0 booting on ESP32-C3; reset_reason=USBPeripheral
I (...) focus_firmware::runtime: application initialized: preset=pomodoro, render=true
I (...) focus_firmware::runtime: state=idle, preset=pomodoro, remaining_ms=1500000
```

The application was flashed with `espflash 4.5.0` using USB reset. The log
confirms that the app boots, obtains the reset reason through the safe ESP-IDF
HAL wrapper, initializes the default Idle/Pomodoro snapshot, and enters the
scheduled event loop. The onboard GPIO8 RGB was not needed for bring-up.

## Purchased interaction hardware

The delivered modules were photographed on 2026-08-08. The repository preserves
the inspection evidence under [`docs/hardware-evidence`](hardware-evidence/).

| Part | Observed marking and pin order | Prototype decision | Remaining bench fact |
| --- | --- | --- | --- |
| Display | 0.96-inch four-pin module; front order `GND`, `VDD`, `SCK`, `SDA`; rear marking `GM009605v43` and flex marking `E0960J-H30-A0` | Power `VDD` from 3.3 V; use `SCK` as I2C SCL and `SDA` as I2C SDA | address, pull-up values, orientation, readable output |
| Encoder | EC11 module; rear order `5V`, `KEY`, `S2`, `S1`, `GND`; multiple `103` (10 kohm) parts and `C1`/`C2` are fitted | Despite the `5V` label, power this passive module from 3.3 V so its onboard pull-ups cannot drive ESP32 inputs above 3.3 V | direction and reliable detent/button behavior at 3.3 V |
| Buzzer | standalone 3 V 12 mm active buzzer packaging identifies polarity; the loose kit buzzer bodies have no legible current marking in the photograph | Use the standalone active buzzer from GPIO1 through 330 ohm, limiting worst-case pin current to about 10 mA; keep both kit alternatives disconnected | short Start and three-pulse Complete cadences physically confirmed at usable volume |
| LED ring | purchased as 24-pixel WS2812 5 V ring; rear pads are `DI`, `5V`, `GND`, `DO` | Disconnect and store for reference; the diameter and reworked connections are unsuitable for this MVP | select a smaller replacement and review its mechanics, power, and data interface in v2 |
| Breadboard power module | `HW-131`; USB-A and barrel inputs, switch, and independent `5V`/`OFF`/`3.3V` rail selectors | Not connected in this prototype | no characterization required unless a later power design proposes using it |

Evidence:

- EC11: [front](hardware-evidence/ec11-front.jpg) and
  [rear](hardware-evidence/ec11-rear.jpg).
- OLED: [front](hardware-evidence/oled-front.jpg) and
  [rear](hardware-evidence/oled-rear.jpg).
- WS2812 ring: [rear pads](hardware-evidence/ws2812-ring-rear.jpg).
- HW-131: [front](hardware-evidence/hw-131-front.jpg) and
  [rear](hardware-evidence/hw-131-rear.jpg).
- Buzzers and loose starter-kit parts:
  [inspection photograph](hardware-evidence/buzzers-and-components.jpg).

The EC11 software starts with ESP32 internal pull-ups, a 25 ms switch debounce,
and an 800 ms long-press
threshold. Quadrature polling accepts only a complete four-transition Gray-code
cycle returning to the boot-time detent. These are host-tested starting values,
not final calibration: the physical module pin order, pull-ups, direction, and
fast/slow detent reliability still require bench validation.

The snapshot presentation model targets four fixed SSD1306 text bands at
vertical positions 0, 13, 28, and 54. It labels the states `READY`, `FOCUS`,
`PAUSED`, and `COMPLETE`, includes the preset and `MM:SS`, and never owns timer
state. A host-tested 24-pixel ring model is retained as an unintegrated
experiment for a later version; the default runtime does not initialize or
update an external ring.

## Starter-kit inventory

- Breadboard power module, 830-point breadboard, 65 jumper wires, two 40-pin
  headers, and 20 female-to-male Dupont wires.
- Precision potentiometer, `4N35`, `74HC595`, active buzzer, passive buzzer, ten
  small buttons, discrete white/yellow/blue/green/red LEDs, and one RGB LED.
- Ceramic capacitors: ten 22 pF and ten 100 nF (`104`).
- Electrolytic capacitors: five 10 uF/50 V and five 100 uF/50 V.
- Resistors: 10/100/220/330 ohm and 1K/2K/5K1/10K/100K/1M values; 220 ohm has
  30 pieces and the other listed values have 10 pieces each.

`74HC595` and `4N35` are not used as WS2812 data buffers. The passive buzzer,
buttons, discrete LEDs, and potentiometer are diagnostic or later-use spares.

## Single-source power decision

The `5V out` header is not a 5 V supply generated by the ESP32-C3. The controller
module itself is a 3.3 V device; while the carrier is USB-powered, `5V out`
exposes the carrier's USB-derived 5 V/VBUS rail for distribution. Because this
unbranded carrier has no available schematic or path rating, the label establishes
the direction and intended use but does not establish an unlimited current output.

The active MVP therefore has exactly one power source:

```text
computer/USB adapter
        |
        v
controller USB port
        |---- onboard 3.3 V regulator ---> ESP32-C3
        |                              `-> OLED + EC11 module

all grounds -------------------------------- common GND
carrier 5V out ------------------------------ unused
HW-131 -------------------------------------- disconnected
external LED ring --------------------------- disconnected
5V in --------------------------------------- disconnected
```

- Use the native `USB` connector as the primary development and power input.
- Connect neither `5V in` nor any HW-131 pin while USB powers the board. This
  removes the uncharacterized parallel/backfeed case rather than relying on the
  HW-131 to isolate it.
- Leave board `5V out` unused in the active MVP. Never route 5 V to ESP32 GPIO,
  OLED logic, the EC11 signals, or the board's 3.3 V rail.
- Route board `3V3` to OLED `VDD` and the EC11 module pin labelled `5V`. On this
  encoder module that pin is the pull-up supply, so 3.3 V is the safe logic-domain
  choice.
- Give every peripheral a common ground. Keep the breadboard's 5 V and 3.3 V
  rails visibly separate.

Measure the active controller/OLED/encoder/buzzer assembly after the buzzer path
is selected. The previous ring current estimates no longer contribute to the
MVP power budget. A later ring version must establish a new adequately rated
5 V topology and must not place HW-131 or a second 5 V supply in parallel with
powered USB.

## Reviewed provisional wiring

| Function | Peripheral pin | Controller pin/rail | Notes |
| --- | --- | --- | --- |
| Encoder supply | `5V` label | `3V3` | deliberately operated at 3.3 V |
| Encoder switch | `KEY` | GPIO5 | module pull-up expected; verify at bench |
| Encoder B | `S2` | GPIO4 | direction finalized after detent test |
| Encoder A | `S1` | GPIO0 | moved from non-responsive GPIO3 path during bench test |
| Encoder ground | `GND` | `GND` | common ground |
| OLED supply | `VDD` | `3V3` | keeps I2C pull-ups in 3.3 V domain |
| OLED clock | `SCK` | GPIO6 | I2C SCL |
| OLED data | `SDA` | GPIO7 | I2C SDA |
| OLED ground | `GND` | `GND` | common ground |
| External LED ring | all pads | not connected | deferred to v2; GPIO10 and `5V out` are unused by the MVP |
| Active buzzer | marked positive | GPIO1 through 330 ohm | bench-verified current-limited direct path; no transistor required |
| Active buzzer | marked negative | `GND` | common ground; active buzzer only |

GPIO2, GPIO8, and GPIO9 remain unused because they are boot-strapping pins;
GPIO8 also owns the onboard RGB. GPIO18/19 remain native USB, GPIO20/21 remain
serial, and GPIO3 remains unallocated after it failed to detect either EC11
phase during the wire-swap bench test. GPIO10 and the carrier `5V out` are also
unallocated. `74HC595` and `4N35` are not suitable future WS2812 data buffers;
the replacement ring's interface will be reviewed from scratch.

## Evidence still required

### Archived WS2812 exploration — 2026-08-08

This evidence explains the retained diagnostic code. It is not part of the
active MVP wiring or acceptance path; the ring is disconnected.

- The `ring-diagnostic` firmware started with an all-off frame and alternated
  pixel 0 at blue `4/255` with all pixels off every two seconds. Serial output
  matched the commanded phases, and the user visually confirmed the expected
  one-pixel result on the wired ring.
- The successful first stage establishes the provisional direct
  `GPIO10 -> 330 ohm -> DI` path, common ground, and USB-derived `5V out` path
  at one-pixel load. It does not establish the final current budget or remove
  the requirement for local decoupling.
- The separate `ring-full-diagnostic` firmware was then built, flashed, and
  observed in serial output commanding all 24 pixels at blue `4/255`, followed
  by all-off, every two seconds. The user visually confirmed that all 24 pixels
  illuminate for the commanded two-second interval and then turn off together.
  This passes the low-current data-chain stage but not the capped application
  profile or measured-current stage.
- The user reports installing the initial local 100 uF electrolytic and 100 nF
  (`104`) ceramic decoupling in parallel across ring 5 V and GND before the
  capped application-profile test.
- The `ring-capped-diagnostic` firmware then cycled the maximum-load application
  frames on all 24 pixels: Running `RGB(0,10,32)`, Paused `RGB(32,8,0)`, and
  Completed `RGB(0,32,4)`, with an all-off interval between each frame. Serial
  output completed repeated cycles without a controller reset, and the user
  visually confirmed correct, stable operation. No `74AHCT125`-class buffer,
  larger bulk capacitor, or alternate 5 V distribution is required for this
  short-wire, USB-powered prototype at the 32/255 channel cap. This conclusion
  does not approve full-brightness white or remove the final current-measurement
  evidence item.

- Record commands, serial logs, measurements, and visible results for each
  peripheral test before the provisional wiring becomes final.

### Active-buzzer staged bring-up — 2026-08-12

- The `buzzer-diagnostic` firmware initialized GPIO1 low, then repeated one
  80 ms Start pulse and a distinct three-pulse Complete cadence without blocking
  its timestamp-driven scheduler.
- The standalone 3 V active buzzer was connected as
  `GPIO1 -> 330 ohm -> +`, with `- -> GND`. The resistor limits theoretical
  worst-case GPIO current to approximately 10 mA.
- Serial output matched each commanded cadence, and the user confirmed that the
  buzzer operated audibly. The current-limited direct GPIO path is therefore
  accepted for this breadboard prototype; the passive buzzer, transistor, and
  protective diode are not required.

### EC11 staged bring-up — 2026-08-08

- With module `5V` powered from 3.3 V and GPIO inputs floating, `KEY` and `S2`
  read high in their open states but `S1` was undefined/low. Enabling the
  ESP32-C3 internal pull-ups established high idle levels for all three inputs.
- Two short presses produced exactly two `Press` events. A separate two-second
  hold produced exactly one `LongPress` event and no duplicate short press.
- On the initial `S1 -> GPIO3`, `S2 -> GPIO4` map, six slow detents produced
  twelve clean transitions only on GPIO4. Swapping the two phase wires at the
  controller still produced transitions only on GPIO4, showing that both EC11
  phase outputs and their jumper wires can signal through GPIO4 while the GPIO3
  input path remains non-responsive.
- The revised map is therefore `S1 -> GPIO0`, `S2 -> GPIO4`, `KEY -> GPIO5`.
  After replacing a loose `S2 -> GPIO4` jumper, both phase inputs produced
  transitions. The first three-detent run captured one complete Gray-code cycle
  but missed the short intermediate phase in two cycles because the default
  100 Hz FreeRTOS tick rounded the requested 1 ms polling delay to 10 ms.
- Firmware now pins `CONFIG_FREERTOS_HZ=1000` so the yielding polling loop samples
  once per millisecond without triggering the idle-task watchdog.
- At the corrected cadence, five slow clockwise detents produced exactly five
  events and five slow counter-clockwise detents produced exactly five events,
  with no duplicates or misses. The phase orientation is finalized as clockwise
  `RotateRight` and counter-clockwise `RotateLeft`.
- The mixed fast run produced exact ten-event clockwise and ten-event
  counter-clockwise segments. It also contained one additional complete
  counter-clockwise event during the operator pause; after resetting the board,
  an isolated ten-detent fast counter-clockwise run produced exactly ten events
  and zero events in the other direction. The isolated counts are the acceptance
  evidence for duplicate/missed-event behavior.
- The final button run again produced exactly two `Press` events and one
  `LongPress` event, with no short event on long-press release. Tasks 4.1 and 4.4
  are accepted on the revised GPIO map and 1 ms polling cadence.

### SSD1306 OLED staged bring-up — 2026-08-09

- The delivered front pin order is `GND`, `VDD`, `SCK`, `SDA`; the module is
  powered from controller `3V3`, with `SCK -> GPIO6` and `SDA -> GPIO7` at
  100 kHz.
- With controller I2C pull-ups disabled, neither `0x3C` nor `0x3D` acknowledged.
  Enabling the ESP32-C3's internal 3.3 V pull-ups produced a stable ACK at
  `0x3C`; the module's own pull-up behavior remains unproven.
- `DisplayRotation::Rotate0` renders upright output. Repeated serial-confirmed
  cycles showed `READY`, `FOCUS`, `PAUSED`, and `COMPLETE` without clipping, and
  the user confirmed readability at normal desk distance.
- The direct Dupont/header connection is mechanically fragile and can lose I2C
  contact when disturbed. This does not change the electrical map, but it must
  be stabilized before final UX acceptance.
- The default integrated runtime then drove the same OLED from live EC11/core
  snapshots. Serial and visual evidence confirmed immediate preset/state
  changes and `90:00`, `89:59`, `89:58`, `89:57` countdown frames on visible
  second boundaries.
- During idle selection, four consecutive `BusWriteError` results were logged
  while the loose contact was disturbed. The accepted core selection remained
  active, retries used the newest snapshot, and rendering recovered without a
  reset. This is the bench evidence for non-blocking state ownership and
  transient OLED retry behavior in task 5.3.
- On 2026-08-12 the default runtime initialized the same current-limited buzzer
  on GPIO1 alongside OLED and EC11. A Press event committed Running, logged
  `buzzer feedback started: Start`, rendered `FOCUS`, and continued visible
  second updates. Subsequent Press/Press/LongPress events produced Paused,
  resumed Running, and Idle without blocking input or monotonic accounting.

### Integrated lifecycle acceptance — 2026-08-13

- The integrated runtime was built with `acceptance-diagnostic`, which preserves
  the production catalog IDs, names, order, default, event loop, adapters, and
  transitions while shortening each duration to 8 seconds.
- Starting from Pomodoro, three clockwise detents rendered Reading, Quick
  Sprint, then Deep Work, proving forward wraparound. One counter-clockwise
  detent rendered Quick Sprint, proving reverse wraparound.
- A short press committed Running and produced the Start pulse. Two rotation
  events while Running did not change the active Focus preset. A short press
  paused at `00:04`; roughly ten seconds of wall time and four more rotation
  events left both preset and remainder unchanged.
- A second short press resumed from `00:04`. The display counted to `00:00`, the
  core emitted one Complete effect, serial logged the Complete cadence, and the
  user heard the distinct three-pulse signal. A short press dismissed Completed
  to Ready with the full `00:08` duration.
- A separate run produced `Press -> Running`, then one `LongPress` event and an
  immediate Ready frame with `00:08`, without a confirmation state or an extra
  short-press event. This completes the on-device evidence for task 6.2; the
  separate 15-, 25-, and 90-minute accuracy runs remain task 6.3.

### Power-cycle recovery acceptance — 2026-08-13

- With production durations restored, the user started Pomodoro, confirmed the
  `FOCUS` screen, disconnected USB power, waited three seconds, and reconnected
  power. The device booted to `READY / Pomodoro / 25:00` instead of restoring
  Running or a reduced remainder.
- In a separate run, the user started Pomodoro, paused it after several seconds,
  disconnected USB power for three seconds, and reconnected it. The device again
  booted to `READY / Pomodoro / 25:00` instead of restoring Paused state.
- Active-session state is therefore confirmed volatile and both Running and
  Paused power-loss cases return safely to Idle, satisfying task 6.5.

### Persistent-selection and fallback acceptance — 2026-08-13

- The user selected Reading and allowed the one-second quiet period to expire.
  After reset, production firmware logged that the settings record was loaded
  from NVS and rendered `READY / Reading / 45:00`, confirming successful save
  and restore by stable preset identifier.
- The `settings-clear-diagnostic` firmware removed the settings record. On the
  next production boot, the missing-record path selected the documented default
  and rendered `READY / Pomodoro / 25:00`; normal short-press starts remained
  available through the same production event loop.
- The `settings-corrupt-diagnostic` firmware then wrote an invalid record. The
  next production boot reported `Decode(BadMagic)` and
  `SettingsFallback(Corrupt)`, selected Pomodoro, and rendered
  `READY / Pomodoro / 25:00` without blocking startup.
- From that corrupt-record fallback, the user short-pressed the encoder and
  confirmed `FOCUS / Pomodoro`, a decreasing countdown, and the Start buzzer
  pulse. A subsequent long press returned the device to `READY`. Saved, missing,
  and corrupt settings therefore all preserve a usable session-start path,
  satisfying task 7.4.

### Offline lifecycle acceptance — 2026-08-13

- The 8-second integrated acceptance build was disconnected from the development
  computer and powered only by a USB power bank. No serial connection, phone,
  companion application, network, cloud service, radio provisioning, or
  wall-clock source was present.
- After autonomous boot, the user selected a preset with the EC11 and completed
  the full Ready, Focus, Paused, resumed Focus, Complete with audible feedback,
  and completion-dismissal lifecycle on the OLED.
- The user confirmed the entire sequence operated correctly before reconnecting
  the device to the development computer. This is the physical evidence for
  task 6.4; the firmware contains no Wi-Fi or Bluetooth setup path.
