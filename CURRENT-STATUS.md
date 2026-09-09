# Current project status

Last updated: 2026-09-09

## Current scope

Three Modbus instruments remain in the active bench-validation scope:

1. Vaisala DPT146 — physical unit available and validated.
2. Vaisala HMD65 — documentation-derived test configuration prepared; no physical unit available.
3. Continental Control Systems WND-M1-MB WattNode Module for Modbus — documentation-derived test configuration prepared; no physical unit available.

The GUI now includes register-table pages for all three Modbus instruments. Each table shows the manufacturer/manual address, zero-based PDU address, data type/word order, access, current readout when known, units, and a technician-facing description.

Documentation-derived CSV register tables now also cover the bridge-compatible or potentially compatible remainder of the intake list: ATI/Badger F12/D12, Micronics U1000MKII-HM, Micronics U3000/UF3300, Precision Digital PD2-6000, Seeed SenseCAP S200, and RKI VOC Pro. These are reference/test artifacts, not bench-validated profiles. The ADAM-4053 is excluded because its useful I/O requires coil/discrete access. The Solair 1100LD is excluded because its serial protocol is Modbus ASCII rather than RTU. See `docs/REMAINING-DEVICE-REGISTER-ASSESSMENT.md` for protocol and revision constraints.

All tracked CSV and native bridge TSV artifacts use plain ASCII without a byte-order mark. Spreadsheet-facing register ranges use spaces around hyphens so Excel does not convert them to dates. Automated tests enforce encoding, headers, safe range text, bridge register classes, and numeric data types.

All other instruments from the original shipment-intake list are deferred.

The present acceptance boundary ends at correctly decoded raw data in Loriot. Existing infrastructure normally handles routing after Loriot; downstream DSP routing is outside this project's current scope.

## Bench hardware identified

### Bridge

- White label: Polygon ExactAire-E5.
- Underlying product: Synetica ENL-MOD-32 enLink Modbus RS-485 Master.
- Firmware: 3.6.
- Capacity: 32 Modbus data points.
- Configuration interface: STM32 USB virtual COM port, observed as COM5.
- Radio: North American hybrid 915 MHz LoRaWAN.
- Power: external 20–24 VDC; USB does not power-cycle the bridge.
- Credentials: DevEUI, AppEUI/JoinEUI, and AppKey are captured in `.secrets/polygon-enl-mod-32-lorawan.env` and intentionally tracked so they transfer with this private repository.
- Firmware 3.6 does not expose a separate provisioned NwkKey in its radio menu.

### Direct RS-485 adapter

- Model: USB-COMi-TB.
- Observed Windows port: COM3.
- Adapter must not act as a second master while the powered bridge is connected to the same RS-485 segment.

## Critical physical findings

- On the DPT146, side-label `CH1` and `CH2` are analog outputs and do not identify physical connectors I and II.
- Physical connector II is the DPT146 RS-485/Modbus port.
- The bridge cable belongs on connector II.
- The USB-COMi-TB was initially wired to the bridge's RS485 OUT terminals; it remained idle during discovery, so no master collision occurred.
- For direct COM3 polling, the bridge master was powered off/isolated.

## DPT146 completed state

Physical instrument:

- Model: Vaisala DPT146.
- Order code: B2MBD102A0X.
- Serial number: U2720970.
- Installed communication: slave ID 1, 19200 baud, 8 data bits, no parity, 2 stop bits.

Validated points:

| Item | Value | PDU address | Bridge type/order |
|---:|---|---:|---|
| 1 | Temperature | 4 | F32 / HL |
| 2 | Dew/frost point | 6 | F32 / HL |
| 3 | Atmospheric-pressure dew/frost point | 10 | F32 / HL |
| 4 | Moisture | 20 | F32 / HL |
| 5 | Absolute pressure | 44 | F32 / HL |
| 6 | Fault status | 512 | U16 / HH |
| 7 | Online status | 513 | U16 / HH |
| 8 | Error code | 515 | U32 / HL |

The ENL-MOD-32 sends the entered address as a zero-based PDU address. Vaisala 32-bit DPT146 values use the low register word first; firmware 3.6 represents this as `HL`.

Validation achieved:

- Direct COM3 Modbus read succeeded.
- Bridge import succeeded for all eight points.
- Detailed bridge test succeeded: 8 reads, 0 exceptions.
- Loriot uplink and payload format were decoded successfully.
- Point-table deletion was validated by importing Slave ID 0 for items 1–8; the bridge reported 0/32.
- Validated-table restoration succeeded for all eight rows.
- Restored readings succeeded: 8 reads, 0 exceptions.
- Bridge reboot persistence succeeded: table remained 8/32 and status remained 8/0.

Last readings during rollback validation were approximately 22.89 °C, 7.36 °C dew/frost point, 7.41 °C atmospheric-pressure dew/frost point, 10276 ppmv moisture, and 1.0095 bara pressure. Status values were fault 1, online 1, and error 0.

Current bridge state: DPT146 validated configuration restored and persistent after reboot.

Primary files:

- `artifacts/bridge-config/vaisala-dpt146-validated.tsv`
- `artifacts/bridge-config/vaisala-dpt146-manifest.yaml`
- `artifacts/bridge-config/vaisala-dpt146-bridge-settings.md`
- `artifacts/bridge-config/vaisala-dpt146-clone-checklist.md`
- `artifacts/bridge-config/enl-mod-32-lorawan-payload.md`
- `artifacts/bridge-config/enl-mod-32-config-reset-restore-validation.md`

## HMD65 prepared state

Status: documentation-derived, testable, not validated, not deployable.

- Profile: `artifacts/device-profiles/to-test/vaisala-hmd65.yaml`.
- ENL-MOD-32 candidate: `artifacts/bridge-config/hmd65-documentation-test.tsv`.
- Point set: eight metric float measurements plus device status, error code, RH measurement status, and temperature measurement status.
- Published PDU addresses are populated.
- Candidate communication: slave ID 1 placeholder; 19200 8N1 when the relevant DIP switches select those values.
- Physical DIP-switch positions determine protocol, address, baud, parity, and termination and must be photographed before import.
- Float word order is the remaining decoding uncertainty. Test `HH` first; if values are implausible, test `HL` without changing addresses.
- Initial test is read-only; configuration and calibration writes remain disabled.

## WND-M1-MB prepared state

Status: documentation-derived, testable, not validated, not deployable.

Exact target: Continental Control Systems WND-M1-MB WattNode Module for Modbus (WND Series), not the WND Wide-Range meter.

- Profile: `artifacts/device-profiles/to-test/wattnode-wnd-m1-mb.yaml`.
- ENL-MOD-32 candidate: `artifacts/bridge-config/wattnode-wnd-m1-mb-documentation-test.tsv`.
- Bound manual: WND-M1-MB-Ref-1.10, documented firmware 1028.
- Point set: total energy; total and per-element active power; three phase-to-neutral voltages; frequency; and three CT currents.
- The selected measurements use native float registers and avoid integer current/power scaling.
- Manual registers are converted to zero-based PDU addresses in the bridge file.
- WND-M1-MB 32-bit registers are low-word-first and use bridge order `HL`.
- Slave ID 1 and 19200 8N1 are placeholders/candidates until the physical front-label options and live settings are inspected.
- CT amp rating is an installation input. Gain and phase adjustments are calibration controls.
- `CurrentIntScale` is a dimensionless full-scale count, not milliamps. At 250 A and scale 20000, resolution is 0.0125 A/count.
- WND-M1-MB manual 1.10 documents PhaseAdjust1..3 defaults of -1000; preserve/read actual values rather than overwriting them.

## Bridge configuration and reset behavior

- Tab-delimited configuration has eight columns: Item, ID, Reg, Addr, Data, Word, Mult, Read.
- Import must omit the header and end with an empty CRLF line.
- Importing an existing item with Slave ID 0 deletes that point.
- A complete point-table reset can be performed by deleting all populated item numbers.
- The firmware menus expose reboot but no labeled global factory-reset function.
- The physical CONFIG button is documented as sending a LoRaWAN status message, not resetting the bridge.
- A global factory reset is not needed for cloning or rollback.

## LoRaWAN state

- Network server: Loriot.
- Join status was confirmed.
- AppEUI/JoinEUI: captured locally; non-secret identifier also recorded in Loriot evidence.
- AppKey: captured in the repository's `.secrets` directory for private clone/recovery use; do not include in technician-facing exports or public materials.
- Interval: 15 minutes.
- Uplink port: 1.
- Receive port: all.
- Public network: on.
- ADR: on.
- Message confirmation: off.
- Data rate observed: DR0 / SF10 / BW125.
- Transmit power: 20 dBm.
- Optional KPI payload fields: off.
- Payload records on firmware 3.6: marker 0x10, zero-based point index, big-endian IEEE-754 float32 value.

## Bridge availability

The physical bridge is not required for current documentation, profile-schema, decoder, simulator, or GUI scaffolding. It can be disconnected and stored with the DPT146 validated configuration installed.

It is required again when:

- an HMD65 or WND-M1-MB becomes available;
- the real technician GUI discovery/import workflow is tested;
- a generated configuration or Loriot payload needs hardware validation;
- another bridge firmware version must be qualified.

## GUI discovery state

- Serial interfaces are rescanned every 1.5 seconds on a background thread.
- USB VID/PID and known bench-port fallbacks identify the bridge and USB-COMi-TB adapter.
- Active Modbus identification is read-only and runs only when the USB-COMi-TB is present without the bridge USB interface; this avoids creating a second master on the bridge bus.
- DPT146 identification requires its pressure, moisture, and status registers to decode with the documented low-word-first layout and plausible values.
- HMD65 identification requires its complete eight-float measurement span plus device, RH, and temperature status layout to agree under one unambiguous word order.
- WND meter-module identification requires the Function 17 manufacturer/family string plus diagnostic model code 530, a nonzero serial number, and a `10xx` firmware version. It tries only addresses 1 and the legacy no-DIP default 127 at 19200 8N1. Protocol evidence remains family-only; the physical `WND-M1-MB` label is required for exact-model confirmation.
- The complete physical test gate, read-only snapshot scope, CT/service decisions, and acceptance sequence are recorded in `docs/WATTNODE-TEST-READINESS.md`.
- Successful direct fingerprints feed live readings into the device detail view.
- COM numbers are no longer treated as device identities. Synetica products sharing USB VID/PID `0483:5740` are distinguished by their unauthenticated console banners.
- A connected enLink IAQ Plus, observed part 003-ADZ-301, identified on COM5 as `FW-AQ-VCP+` firmware 5.06 with North American Hybrid FSB #1 / 915 MHz and its DevEUI captured in the private profile.
- The serial login is deterministically derived from the DevEUI shown in the connection banner: normalize `0004a30b00084f86` and take its final four characters, `4f86`. Background discovery does not submit it; configuration/readout workflows may derive and use it. The initial automation attempt did not advance past the prompt, so terminal input timing/line-ending compatibility still needs implementation validation.
- Radio profiles now default to enabled US915 Hybrid FSB #1, with an explicitly disabled EU868 future stub.
- IAQ Plus authenticated console access is working with DTR on, RTS off, and the DevEUI-derived login.
- The GUI prototype can request current IAQ readings, parse seven engineering/status values, back up Quick Start/radio/configuration pages to JSON, and preview radio profiles without writing the device.
- The observed unit inventory includes a VOC air-quality module and GSS CO2 module LP26/614548 with auto-calibration enabled. Configure Device page 2 and particle details remain to capture.
- IAQ commissioning scope is now credential-first: preserve JoinEUI/AppEUI, provision the selected AppKey, and treat measurements as optional verification. The attached IAQ and stored bridge/project JoinEUI values match; their AppKeys do not. Firmware 5.06 exposes no separate NwkKey.
- Firmware-update preflight is scaffolded: package manifests bind product, firmware code, source/target versions, radio regions, image SHA-256, vendor approval, flash method, and recovery procedure. The GUI can inspect packages, but flashing is blocked because no authoritative IAQ Plus updater/image/bootloader and recovery procedure are currently available.

## Transport reliability finding

Live GUI testing exposed a structural failure in the current PowerShell console helper. It starts a new process, sleeps for fixed intervals, and requires an exact menu prompt. When the ENL-MOD-32 is already authenticated, remains in a submenu, echoes differently, or returns output in different chunks, a healthy device can be reported as an authentication or menu failure. The latest observed failure expected `Modbus Configuration Menu:` and stopped without a verified write.

The production direction is therefore a persistent Rust hardware agent behind the current Python/Tk GUI. It will exclusively own each COM port, recognize accumulated console state, recover from known menus, serialize discovery and actions, and return structured JSON progress/results. Existing PowerShell helpers are retained only as bench diagnostics while paths are migrated. The detailed design is `docs/RUST-HARDWARE-AGENT.md`.

The corrected CSV/TSV delivery bundle is `artifacts/modbus-csv-tsv-bundle.zip`. It contains 21 strict-ASCII artifacts: nine register-table CSVs, ten bridge TSVs, and two private native bridge backups. The older firmware export is retained only under the explicit name `enl-mod-32-firmware-3.6-historical-precorrection-export.tsv`, so it cannot be mistaken for the validated DPT146 table.

## Next work

Without hardware:

1. Scaffold `modbus-agent.exe` and its versioned JSON Lines command/event contract.
2. Add replay fixtures for bridge login, prompt fragmentation, stale submenus, native export, and Read All.
3. Move passive Synetica discovery and read-only ENL-MOD-32 actions behind one exclusive per-port actor.
4. Integrate agent progress, results, and plain-language error codes into the existing GUI.
5. Implement the profile loader and validation rules.
6. Turn pre-made selection into a guarded programming workflow: Preflight, Preview, Confirm, Program, Read Back, and Validate.
7. Move direct Modbus polling and native IAQ console access behind the agent.
8. Add safe device-configuration writers only for fields classified as installation settings.
9. Add live engineering-value confirmation and a commissioning pass/fail export.
10. Keep writes disabled by default and require backup, diff, confirmation, readback, and recovery evidence.
11. Add the final DSP how-to only after the Modbus and Loriot workflow is proven for all three active devices.

When HMD65 or WND-M1-MB hardware arrives:

1. Photograph identity, firmware, wiring, power, and communication settings.
2. Run the profile's read-only bench sequence.
3. Resolve remaining serial/word-order assumptions.
4. Compare values with a trusted reference or plausible stimulus.
5. Import the candidate bridge table, read back, and validate all points.
6. Capture and decode a Loriot payload.
7. Promote the profile only after evidence review.
