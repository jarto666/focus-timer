# SuperMini TFT integration evidence — 2026-08-17

## Hardware and production profile

- Controller: ESP32-C3 SuperMini, revision v0.4, 4 MB flash.
- Display: 2.0-inch 240x320 ST7789V SPI TFT.
- TFT wiring: `SCLK GPIO6`, `MOSI GPIO7`, `RES GPIO3`, `DC GPIO4`,
  `CS GPIO10`, `VCC 3V3`, `BLK 3V3`, common ground.
- Remaining production wiring: EC11 `S1 GPIO0`, `S2 GPIO20`,
  `KEY GPIO5`; active buzzer `GPIO1`.

## Automated gates

The repository device check passed after the integration. The additional
TFT/worker host gate passed warning-denied Clippy and 21 tests: four worker
tests, twelve presentation tests, and five renderer tests. Strict OpenSpec
validation also passed.

The ESP target build matrix passed for production plus `tft-diagnostic`,
`oled-diagnostic`, `encoder-diagnostic`, `buzzer-diagnostic`,
`acceptance-diagnostic`, both settings diagnostics, all three journal
diagnostics, `ble-echo-diagnostic`, `ble-fault-diagnostic`,
`radio-failure-diagnostic`, and `tft-failure-diagnostic`.

## Live measurements

The last pre-TFT combined-production evidence was 1,094,288 bytes (26.50% of
the 4,128,768-byte app partition). The integrated TFT production image is
1,181,072 bytes (28.61%), a delta of +86,784 bytes; 2,947,696 bytes remain in
the app partition.

The acceptance runtime reported 96,748 bytes of free 8-bit heap before the
first TFT frame. After one deliberately injected transfer failure and a
successful bounded retry it reported:

- free/minimum 8-bit heap: 96,560 / 96,560 bytes;
- display-worker minimum stack reserve: 5,528 bytes from its 32 KiB stack;
- complete animated 240x320 frame: 197 ms (about 5.1 frames/s).

The implementation allocates one eight-row RGB565 transfer buffer of 3,840
bytes, a 4,096-byte procedural noise texture, and a 512-byte RGB565 palette.
It does not allocate a 153,600-byte full-screen RGB565 framebuffer. Relative
to the last OLED+BLE acceptance checkpoint, idle free heap decreased by 40,964
bytes, primarily the explicit 32 KiB worker stack and thread overhead, while
leaving about 96 KiB for BLE, journal, and runtime work.

## Fault and stability checkpoint

The `tft-failure-diagnostic` build booted the production NVS, journal, BLE,
timer, input, and buzzer runtime, initialized the TFT worker, injected exactly
one `InjectedTransfer` error, retained the newest generation, and completed a
frame after bounded backoff. The monitor showed no panic, watchdog reset, or
reboot loop. A normal production image was rebuilt and flashed afterward
without erasing NVS.

## Still requiring physical acceptance

The combined `acceptance-diagnostic` was subsequently exercised with the TFT,
EC11, and active buzzer connected. Boot observed high idle levels on GPIO0,
GPIO20, and GPIO5. Clockwise and counter-clockwise detents produced
`RotateRight` and `RotateLeft`; the display changed promptly during continuous
animation. Start, pause, resume, completion, dismissal, and long-press cancel
all agreed across input and TFT. Start produced its single pulse and completion
its three-pulse cadence. NVS selection persistence committed successfully and
the completed eight-second session appended journal sequence 6. No watchdog or
reset occurred during the run.

The connected-iPhone stress pass, loose/unplugged-display behavior, and
whole-device USB current remain physical checks. Battery power, soft power-off,
OTA, enclosure work, and the LED ring remain explicitly outside this change.

After acceptance, the normal production image was flashed back onto the board.
It booted cleanly with persisted settings, healthy journal bounds 1 through 6,
BLE advertising, and the TFT worker active; no diagnostic timing overrides
remain on the device.
