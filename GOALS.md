# Development requirements

Reviewed 2026-09-10. [Current support and evidence](CURRENT-STATUS.md) is the implementation handoff. A passing software test does not close a physical acceptance item.

## Completed

- [x] Native Windows application with live data, dynamic USB routes and persistent E5 session handling.
- [x] DPT146 reads through E5 bridge and FTDI USB adapter on the local bench.
- [x] E5 point-table program/backup/restore with exact readback; save/load and backup-history GUI acceptance.
- [x] Quiet background polling, queued foreground actions and fixed status bar.
- [x] Retained readings/timestamps; adapter detail sensor readings; history gaps/CSV and labeled chart scales.
- [x] Unified configuration view, unavailable-target banner, responsive register row heights and consistent columns.
- [x] Dedicated troubleshooting, system theme/region units and safe polling toggle.
- [x] Polygon branding, Brandon font, seven device photos and vector application icon.
- [x] Offline device manuals; HMD65 alternate float bank; ATI F12/PAA E5 candidate and status decoding.
- [x] Approximately 90% coverage with focused regressions; latest combined test/offline-GUI measurement 92.40%.

## Hardware acceptance still open

- [ ] Full E5 power-loss persistence: remove both USB and external supply, then compare fresh export. Prior reboot/USB tests do not establish this.
- [ ] Overnight soak with failure/recovery timing recorded.
- [ ] Broader unplug/replug, route changes, startup recovery and cancellation combinations. Basic adapter reads and handoff are already verified.
- [ ] Clean second Windows PC package/driver acceptance.
- [ ] HMD65 metric/non-metric word order and physical read validation.
- [ ] WattNode identity, CT/service mapping and readings.
- [ ] ATI F12/PAA units/range, serial settings, readings/status and E5 profile validation.

## Implementation and inputs still open

- [ ] Work toward full coverage with lean behavioral tests; remaining native driver/dialog and state-machine branches are tracked in docs/RUST-COVERAGE.md.

- [ ] Native IAQ Plus identification, readings and protected configuration-backup workflows.
- [ ] Additional Synetica model profiles and device-specific console behavior.
- [ ] Direct ATI adapter identification/polling.
- [ ] Firmware image and exact target model/revision; verified address/range and erase policy.
- [ ] Firmware preflight, protected settings preservation, exclusive transport handoff, CLI programming/verification, rediscovery and recovery tests.
- [ ] Full E5 and IAQ Plus user-guide PDFs; separate ATI Modbus PDF.
- [ ] Review remaining parity items before declaring a complete Python replacement.

Full historical checklists and progress are preserved under docs/archive/2026-09-10-before-documentation-pass/. Previously supplied artwork and completed adapter acceptance are no longer listed as missing.
