# Development requirements

Reviewed 2026-09-16. [Current support and evidence](CURRENT-STATUS.md) is the implementation handoff. A passing software test does not close a physical acceptance item.

## Completed

- [x] Network feedback: top-left slave badges, configured card ordering, 32-device/32-point limits, stable register selection, early table/partial read display, per-slave failure notices, scan budgeting and programming preflight recovery.

- [x] Native Windows application with live data, dynamic USB routes and persistent Modbus Bridge session handling.
- [x] DPT146 reads through Modbus Bridge and FTDI USB adapter on the local bench.
- [x] Modbus Bridge point-table program/backup/restore with exact readback; save/load and backup-history GUI acceptance.
- [x] Quiet background polling, queued foreground actions and fixed status bar.
- [x] Retained readings/timestamps; adapter detail sensor readings; history gaps/CSV and labeled chart scales.
- [x] Unified configuration view, unavailable-target banner, responsive register row heights and consistent columns.
- [x] Dedicated troubleshooting, system theme/region units and safe polling toggle.
- [x] Polygon branding, Brandon font, seven device photos and vector application icon.
- [x] Offline device manuals; HMD65 alternate float bank; ATI F12/PAA Modbus Bridge candidate and status decoding.
- [x] Approximately 90% coverage with focused regressions; current headless results are recorded in docs/RUST-COVERAGE.md.

## Hardware acceptance still open

- [ ] Full Modbus Bridge power-loss persistence: remove both USB and external supply, then compare fresh export. Prior reboot/USB tests do not establish this.
- [ ] Overnight soak with failure/recovery timing recorded.
- [ ] Broader unplug/replug, route changes, startup recovery and cancellation combinations. Basic adapter reads and handoff are already verified.
- [ ] Clean second Windows PC package/driver acceptance.
- [x] HMD65 operation confirmed; hardware and firmware versions not recorded.
- [ ] Complete HMD65 metric/non-metric register-set and version coverage.
- [x] WattNode WND-M1-MB operation confirmed; hardware and firmware versions not recorded.
- [ ] WattNode complete register-set coverage and installation-specific current-transformer/service validation.
- [x] ATI F12 transmitter hardware 1.01 / software 1.25 confirmed working.
- [ ] ATI gas-module identification, units/range and complete register-set validation.

## Implementation and inputs still open

- [ ] Work toward full coverage with lean behavioral tests; remaining native driver/dialog and state-machine branches are tracked in docs/RUST-COVERAGE.md.

- [ ] Native IAQ Plus identification, readings and protected configuration-backup workflows.
- [ ] Additional Synetica model profiles and device-specific console behavior.
- [ ] Direct ATI adapter identification/polling.
- [ ] Firmware image and exact target model/revision; verified address/range and erase policy.
- [ ] Firmware preflight, protected settings preservation, exclusive transport handoff, CLI programming/verification, rediscovery and recovery tests.
- [ ] Full Modbus Bridge and IAQ Plus user-guide PDFs; separate ATI Modbus PDF.
- [ ] Review remaining parity items before declaring a complete Python replacement.

Full historical checklists and progress are preserved under docs/archive/2026-09-10-before-documentation-pass/. Previously supplied artwork and completed adapter acceptance are no longer listed as missing.
