# Application support status

Reviewed against source and tests: 2026-09-10. This page is the current handoff; dated evidence records past observations, not current physical connectivity.

## Deliverable

Polygon Device Configurator is a Windows-first Rust/egui application. Latest executable package: [175644](artifacts/releases/Polygon-Device-Configurator-windows-x64-20260910-175644.zip). Python and PowerShell are development/prototype tools, not runtime requirements. Later documentation changes do not require rebuilding this package.

## Device support

| Device | Native implementation | Hardware evidence / remaining work |
|---|---|---|
| E5 bridge (Synetica ENL-MOD-32, firmware 3.6) | Persistent console, live reads, point-table backup/program/restore and readback | DPT146 program/restore and repeated reads passed; full power-loss persistence and overnight soak pending |
| Vaisala DPT146 | E5 profile and direct USB adapter reads | Both routes verified locally; real last-good values and timestamps retained |
| Vaisala HMD65 | Metric/non-metric float profiles; direct adapter support | Physical verification pending; HH E5 float order remains candidate |
| WattNode WND-M1-MB | Native-float E5 profile; direct adapter support | Physical verification pending; CT/service mapping must be checked |
| ATI F12/PAA | E5 profile, register map, fault decoding and product photo | Hardware verification and gas units/range pending; direct ATI adapter polling unimplemented |
| Synetica enLink IAQ Plus | Reference/manual availability only | Python console observations are historical; native readings and backup port pending |

Last local hardware acceptance: E5 hardware switch off with external supply retained, FTDI USB adapter on shared RS-485 wiring reading the DPT146, eight values and zero errors. This is a recorded bench state, not a live inventory. COM3/COM5 in evidence are observations, never defaults.

## Implemented operator behavior

- USB identities drive route selection. Automatic polling waits five seconds after completion, with failure backoff; it is not a fixed 10-second sampling clock.
- One persistent E5 console handle; receive recovery reasserts unchanged serial settings, not a physical device reset or blind command resend.
- Manual operations can queue behind background reads. Poll progress stays quiet; foreground/queued state uses a fixed bottom status bar.
- Devices retain last-good data with readable local timestamps and stale/error labels. The adapter page shows its attached sensor data. Configured E5 point tables do not prove physical sensor identity; warnings identify failures/implausible data, not guaranteed model detection.
- Configuration offers five profiles or a TSV, a prominent unavailable-target banner, review, program, backup and save/load. Programming requires a fresh matching review, durable point-table backup, acknowledged writes and exact export verification. There is no automatic rollback.
- Backups cover the E5 point table only: not serial framing, radio credentials, calibration or firmware.
- History records fresh acquisitions only, up to 50,000 point samples in this application session. Failed samples create gaps. Chart labels include measurement, native units and numerical value/time scales. CSV export preserves native values; history is not automatically restored after restart.
- Settings persist system/light/dark appearance, system/US/UK/EU display units and automatic polling. Turning polling off lets an in-flight operation finish. Individual unit overrides and native/display comparison are presentation controls.
- Seven offline PDF manuals and seven device photographs are bundled. Full E5 and IAQ Plus guides remain unavailable. Program icon is the vector icon with PNG/ICO exports.

## Firmware work

Firmware integration is planned, not implemented. Required image, target, preservation, and recovery checks are tracked in [firmware design](docs/FIRMWARE-UPDATES.md). Private device-specific procedures remain local.

## Validation and remaining acceptance

151 tests and warnings-denied Clippy pass; combined test/offline-GUI line coverage is 92.40% (full source scope with limitations in [coverage](docs/RUST-COVERAGE.md)). Most recent full UI package check captured 23 offline views on this PC for build 175404; build 175644 adds the traced icon and passed package compilation/dependency inspection. Offline results are not hardware qualification.

Remaining: full power-loss persistence; extended soak; broader USB recovery combinations; clean second Windows PC; HMD65, WattNode and ATI hardware verification; native IAQ workflows; firmware implementation and recovery. See [goals](GOALS.md) and [operator guide](docs/OPERATOR-GUIDE.md).
