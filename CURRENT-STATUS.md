# Application support status

Network feedback changes reviewed against source and offline tests: 2026-09-16. This page is the current handoff; dated evidence records past observations, not current physical connectivity.

## Deliverable

Polygon Device Configurator is a Windows-first Rust/egui application. Use the root **Polygon Device Configurator** shortcut or `tools/Start-LatestBuild.ps1` for the latest local executable. Python and PowerShell are development/launch tools, not executable runtime requirements. Local builds are separate from published GitHub releases.

## Device support

| Device | Native implementation | Hardware evidence / remaining work |
|---|---|---|
| Modbus Bridge (Synetica ENL-MOD-32, firmware 3.6) | Persistent console, live reads, point-table backup/program/restore and readback | DPT146 program/restore and repeated reads passed; full power-loss persistence and overnight soak pending |
| Vaisala DPT146 | Modbus Bridge profile and direct USB adapter reads | Both routes verified locally; real last-good values and timestamps retained |
| Vaisala HMD65 | Metric/non-metric float profiles; direct adapter support | Operation confirmed 2026-09-16; hardware/firmware versions and tested register selection not recorded |
| WattNode WND-M1-MB | Native-float Modbus Bridge profile; direct adapter support | Operation confirmed 2026-09-16; hardware/firmware versions not recorded; CT/service mapping remains installation-specific |
| ATI F12/PAA | Modbus Bridge profile, register map, fault decoding and product photo | Operation confirmed 2026-09-16; transmitter hardware 1.01 / software 1.25 confirmed; tested gas module not recorded; direct ATI adapter polling unimplemented |
| Synetica enLink IAQ Plus | Reference/manual availability only | Python console observations are historical; native readings and backup port pending |

Last local hardware acceptance: Modbus Bridge hardware switch off with external supply retained, FTDI USB adapter on shared RS-485 wiring reading the DPT146, eight values and zero errors. This is a recorded bench state, not a live inventory. COM3/COM5 in evidence are observations, never defaults.

## Implemented operator behavior

- USB identities drive route selection. Automatic polling waits five seconds after completion, with failure backoff for recoverable errors; console timeouts pause polling; it is not a fixed 10-second sampling clock.
- One persistent Modbus Bridge console handle; receive recovery reasserts unchanged serial settings, not a physical device reset or blind command resend.
- Manual operations can queue behind background reads. Poll progress stays quiet; foreground/queued state uses a fixed bottom status bar.
- Devices retain last-good data with readable local timestamps and stale/error labels. The adapter page shows its attached sensor data. Configured Modbus Bridge point tables do not prove physical sensor identity; warnings identify failures/implausible data, not guaranteed model detection.
- Configuration offers five profiles or a TSV, a prominent unavailable-target banner, review, program, backup and save/load. Programming requires a fresh matching review, durable point-table backup, acknowledged writes and exact export verification. There is no automatic rollback.
- Backups cover the Modbus Bridge point table only: not serial framing, radio credentials, calibration or firmware.
- History records fresh acquisitions only, up to 50,000 point samples in this application session. Failed samples create gaps. Chart labels include measurement, native units and numerical value/time scales. CSV export preserves native values; history is not automatically restored after restart.
- Settings persist system/light/dark appearance, system/US/UK/EU display units and automatic polling. Turning polling off lets an in-flight operation finish. Individual unit overrides and native/display comparison are presentation controls.
- Seven offline PDF manuals and seven device photographs are bundled. Full Modbus Bridge and IAQ Plus guides remain unavailable. Program icon is the vector icon with PNG/ICO exports.

## Firmware work

Firmware integration is planned, not implemented. Required image, target, preservation, and recovery checks are tracked in [firmware design](docs/FIRMWARE-UPDATES.md). Private device-specific procedures remain local.

## Validation and remaining acceptance

Release checks on 2026-09-16 passed Rust formatting, warnings-denied Clippy, 197 Rust tests, 43 Python tests and the generated-reference consistency check. Headless Rust line coverage is 90.53%, above the 90% floor. Offline package checks verify the executable, debug symbols and 34 rendered views. See [coverage](docs/RUST-COVERAGE.md) for measured results. Offline results are not hardware qualification.

Remaining: full power-loss persistence; extended soak; broader USB recovery combinations; clean second Windows PC; complete register-set and version coverage for HMD65, WattNode and ATI; native IAQ workflows; firmware implementation and recovery. Confirmed working models and known versions are listed above. See [goals](GOALS.md) and [operator guide](docs/OPERATOR-GUIDE.md).

## Network feedback update — 2026-09-16

- Device cards use vertical layouts and top-left slave badges, preserving configured order. Each point remains associated with its slave.
- Up to 32 device entries share the 32-point table limit; register selection stays expanded while editing.
- Exported tables and validated partial point results reach the interface before the full scan completes. Per-slave failures remain visible until fresh outcomes replace them.
- Automatic scan budgeting accounts for table size and reported retries. Console failures pause polling, with a configuration-only recovery action; preflight failures release the programming lock, uncertain writes do not.
- User-facing bridge name: Modbus Bridge.
- Table-based profile display is immediate after export; persistent EUI-based profile association is not implemented.
- Faulty-power behavior on the remote multi-slave network still needs hardware verification. No local hardware configuration was changed for this update.
