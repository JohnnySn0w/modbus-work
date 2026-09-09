# Polygon/Synetica bridge configuration artifacts

## Historical pre-correction export

`enl-mod-32-firmware-3.6-historical-precorrection-export.tsv` is the configuration exported read-only from the Polygon ExactAire-E5 / Synetica ENL-MOD-32 before this project corrected its DPT146 point mapping.

Bridge-wide Modbus settings observed with this export:

- Firmware: 3.6
- Baud: 19200
- Data bits: 8
- Parity: None
- Stop bits: 2
- Retries: 1
- Timeout: 500 ms
- Inter-message delay: 150 ms
- Configured data points: 8/32

This file records the original state only. Do not import it as a DPT146 profile: its measurement addresses are one register too high, its 32-bit word settings do not match the tested DPT146, and its final error-code address is also wrong. A read-all test produced timeouts on all eight points.

`vaisala-dpt146-direct-validation.md` records the successful read-only direct poll through COM3. It confirms slave ID 1 and serial format 19200 8N2, plus all documented measurements and status values.

## Critical DPT146 installation note

`CH1` and `CH2` on the DPT146 side label are analog outputs; they do **not** mean physical port I and port II. Connector I carries the analog channels. Connector II carries RS-485/Modbus. The bridge cable must be plugged into connector II. Both are similar 4-pin M8 connectors and use the same wire colors, so always verify the Roman-numeral socket marking.

## Planned deliverables

- `vaisala-dpt146-validated.tsv` — validated, clone-ready import table for ENL-MOD-32 firmware 3.6.
- `vaisala-dpt146-manifest.yaml` — machine-readable compatibility and validation manifest; currently requires exact firmware 3.6.
- `vaisala-dpt146-bridge-settings.md` — corrected bridge-wide and per-point settings.
- `vaisala-dpt146-validation.md` — identity, live readings, test evidence, and acceptance result.
- `vaisala-dpt146-clone-checklist.md` — technician-focused import and verification procedure.

## Validation status

The validated table was imported into the Polygon/Synetica bridge and verified using its detailed read function: 8 successful reads, 0 exceptions. The bridge now contains the corrected configuration.

The point-table reset and rollback workflow has also been validated. Firmware 3.6 deletes an item when a tab-delimited import row uses Slave ID `0`. All eight points were deleted, the validated table was restored, all reads passed, and the restored configuration persisted across a bridge reboot. See `enl-mod-32-config-reset-restore-validation.md`.

The remaining `*-documentation-test.tsv` files are manufacturer-documentation-based starting points. Their status and serial assumptions are listed in `documentation-test-profiles.md`; none should be promoted to a validated profile until it passes a bench read test.

The GUI writer workflow was validated live on 2026-09-01: pre-write export, acknowledged replacement, exact exported readback, and Read All verification completed against firmware 3.6. See `enl-mod-32-gui-writer-validation.md`.
