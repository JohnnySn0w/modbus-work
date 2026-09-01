# Prototype design decisions and open questions

This file records decisions made autonomously for the rough technician prototype. None of the open questions block continued read-only development.

## Decisions made

1. **One application, two device paths.** Modbus instruments are commissioned directly or through a bridge adapter. Native Synetica LoRaWAN sensors such as the IAQ Plus use their USB console adapter and do not pretend to be Modbus devices.
2. **COM numbers are never identities.** USB metadata selects a candidate transport; a protocol/banner fingerprint establishes the product. Synetica USB VID/PID 0483:5740 is shared and therefore insufficient by itself.
   The FTDI 0403:6001 identity likewise selects only a candidate serial transport. Neither a remembered COM number nor a particular USB serial number identifies the Modbus instrument connected behind it.
3. **US is the operational default.** `us915_hybrid_fsb1` is enabled and selected by default. `eu868` exists as a disabled profile until matching hardware/firmware and Loriot configuration are qualified.
4. **Discovery does not authenticate.** The 1.5-second watcher reads and caches the unauthenticated banner once per USB instance. Login and menu reads happen only after a technician requests them.
5. **The enLink login is derived, not stored as product data.** Normalize the displayed DevEUI and use the final four hexadecimal characters. A derived value may be included in a private recovery artifact, but the GUI need not display it.
6. **Bridge writes are compatibility-gated and verified.** ENL-MOD-32 point-table programming is enabled only for the validated model/firmware identity `ENL-MOD-32` 3.6. The workflow saves the live export, replaces the table through acknowledged console imports, compares an exported readback, and runs Read All Data Points. If verification fails, the saved live export is retained as the recovery artifact and the GUI does not claim success. Other bridge firmware remains read-only.
7. **Secrets travel only in explicitly private artifacts.** This repository is intentionally private and carries recovery credentials per project direction. Normal logs, redacted evidence, and routine UI screens do not display keys.
8. **Firmware is part of compatibility identity.** An IAQ configuration is keyed by product family plus firmware code/version and radio region, just as bridge configurations are keyed by model plus firmware.
9. **Unexpected readings are shown, not silently corrected.** The observed GSS CO2 value is parsed but called out as unvalidated. Plausibility warnings belong beside the raw engineering value.
10. **Windows is the first supported bench platform.** The live enLink helper uses PowerShell/.NET serial behavior because it is reliable with this CDC device. Parsing and profile logic remain pure Python and replay-testable.
11. **Firmware is a separately authorized workflow.** A package must pass exact identity, region, upgrade-path, hash, vendor-approval, and recovery checks. Configuration authorization never implies permission to flash firmware.

## Open design questions for later review

1. Is region selection actually writable on all enLink IAQ Plus hardware, or is EU868 versus US915 fixed by orderable hardware/firmware? Until confirmed, the UI treats region profiles as compatibility selections and does not issue a region write.
2. What sensors and values appear on Configure Device page 2 for part 003-ADZ-301, especially particle bins and counts? This should be captured before defining the full live-readout schema or payload decoder.
3. **Resolved:** normal IAQ commissioning preserves the existing JoinEUI/AppEUI and provisions only the selected AppKey. Measurement display is useful but not required for credential commissioning. Calibration, particle-cleaning, and advanced radio controls remain expert-only.
4. Which AppKey credential profile should be the default for IAQ commissioning? The attached IAQ currently has a different AppKey from the stored bridge/project AppKey, although their JoinEUI values already match. No physical key write should occur until that target is explicitly selected.
5. Should the GUI store backups beside the repository, in a per-job commissioning folder, or in an organization-managed record system? The prototype uses operator-selected JSON files.
6. What acceptance ranges should be used for IAQ channels? These should come from the exact product datasheet and commissioning procedure, not generic indoor-air assumptions.
7. Does Loriot already have a decoder for every optional 003-ADZ-301 channel, and can its decoder/profile be exported for a clone package? DSP remains downstream and out of the current proof-of-concept boundary.

## Next implementation slice

- Capture Configure Device page 2 and particle options without writes.
- Add a plain-language credential diff that always marks JoinEUI/AppEUI as preserved and shows only whether the AppKey will change, without revealing it.
- Implement writes one field at a time with backup, confirmation, reboot when required, readback, and rollback evidence.
- Add Loriot payload replay fixtures and compare them with USB live readings.
- Package the Windows application for technicians who do not have Python installed.
