# Modbus Shipment Intake and Bridge Configurator

## Purpose

Build a Windows desktop application and repeatable commissioning process that lets operations technicians:

1. connect a supported Modbus instrument;
2. identify or select its device profile;
3. verify communications and preview every supported reading;
4. assign those readings to bridge channels;
5. validate LoRa transmission; and
6. save an auditable configuration/report for shipment and downstream DSP integration.

The governing product workflow is **plug in → identify → select an approved pre-made configuration → back up → preview → program → read back → confirm live data → validate Loriot → export**. The GUI is an auto-configuration tool first; dashboards and downstream DSP work are secondary.

The first working target is a Vaisala DPT146 connected as a Modbus RTU slave to channel 1 of a Polygon-branded LoRa Modbus master bridge. The longer-term target is a profile-driven tool that supports every product in the device backlog without adding one-off GUI code for each model.

### Current scope boundary

Downstream DSP integration is deferred until the initial Modbus device proofs of concept are complete. A device POC is currently accepted when direct Modbus communication, bridge configuration/readback, and Loriot raw-payload decoding are validated. DSP routing, field naming, and application-level semantics are a later phase and must not block device-profile or configurator development.

## Current bench setup

### Hardware observed

- Windows laptop.
- USB-COMi-TB USB-to-RS-485 adapter.
  - Windows port: `COM3`.
  - USB interface: FTDI FT232-family (`VID 0403`, `PID 6001`).
- Polygon Modbus Master Bridge, strongly identified as a white-labelled or derivative Synetica enLink LoRaWAN Modbus Master.
  - Windows USB configuration port: `COM5`.
  - USB interface: STM32 Virtual COM Port (`VID 0483`, `PID 5740`).
  - Physical features matching the enLink hardware: DIN-rail enclosure, micro-USB configuration, Status and Mode LEDs, Config button, power switch, RS-485 IN/OUT, and 12–24 VDC input.
- Fused DIN-rail power assembly with XP Power AC-to-24 VDC supply.
- Vaisala DPT146 dewpoint and pressure transmitter connected to the bridge's RS-485 side and represented as channel 1 in the current bridge configuration.

### Important topology note

The bridge is the Modbus RTU master and the DPT146 is a slave. Do **not** attach COM3 as another active master to the same energized RS-485 bus. For direct laptop polling, first power down or electrically isolate the bridge from that RS-485 segment, then connect the USB-COMi-TB. Two masters transmitting on the same bus can cause collisions and misleading results.

The USB-COMi-TB was physically attached to the bridge's RS485 OUT terminals during initial discovery. This means it was electrically present on the same bus, but the discovery commands only queried Windows device metadata: they did not open COM3 or transmit serial data. No Modbus-master collision was caused by this project work. A connected but idle adapter can still affect the bus if its termination, bias, or operating-mode switches are wrong, so those settings must be documented before active testing.

## Identification confidence

The bridge enclosure and interfaces match the Synetica enLink Modbus Master hardware guide closely enough to use that guide for electrical and UI expectations. The Polygon branding may carry different firmware, channel limits, payload encoding, or configuration software, so the exact model and firmware remain commissioning unknowns.

### Photo-based confirmation (2026-08-28)

The observed Polygon ExactAire-E5 unit is confirmed as Synetica enLink hardware with **high confidence**, based on this combination of distinctive matches:

- identical 86 x 70 x 57 mm DIN-rail enclosure and front-panel geometry;
- the exact terminal sequence `RS485 IN C B A`, `RS485 OUT C B A`, `VIN + -`;
- RS485 IN and OUT blocks in the documented positions;
- micro-USB configuration connector in the documented position;
- `STATUS`, `MODE`, and `CONFIG` indicators/button in the documented layout;
- top power switch and `ON / POWER` marking;
- integral/external antenna arrangement;
- 24 VDC supply, within Synetica's documented 12–24 VDC input range;
- STM32 USB virtual COM interface on COM5, consistent with the embedded configuration port;
- Polygon branding explicitly calls it a “Modbus Master Bridge,” matching the enLink product function.

The remaining uncertainty is not the base hardware identity; it is the Polygon-specific firmware/configuration layer. Until firmware and menus are read, use **“Polygon ExactAire-E5, based on Synetica enLink Modbus Master hardware”** as the precise project description.

### USB identity confirmation (2026-08-28)

A non-destructive terminal session on COM5 returned the native Synetica banner and removes the remaining hardware-identification uncertainty:

- Region: North American band (Hybrid), 915 MHz
- Model number: `ENL-MOD-32`
- Model name: `enLink Modbus RS485 RTU Master`
- Firmware: `3.6`
- DevEUI: recorded in the controlled bench log; do not treat LoRaWAN session/application keys as project documentation

The unit is therefore a Polygon-labelled Synetica ENL-MOD-32, not merely a visually similar design.

### Existing bridge configuration observed read-only

- Serial: 19200 baud, 8 data bits, no parity, 2 stop bits (`8N2`)
- Retries: 1
- Timeout: 500 ms
- Inter-message delay: 150 ms
- Configured data points: 8 of 32
- Slave ID: 1 for all eight points
- Configured holding-register starts: 5, 7, 11, 21, 45, 512, 513, and 514
- Data types: five F32 values, followed by two U16 values and one U32 value
- A single “Read All Data Points” test returned timeout exception 12 for all eight points.

No settings were changed or saved. The terminal session was logged off afterward.

The configured measurement registers are recognizably derived from the DPT146 map, but the bridge serial format does not match the DPT146 documented factory Modbus default (`19200 8E1`). The bridge uses `8N2`. Before changing either side, verify the DPT146's current settings or test it in isolation through COM3. Also verify slave ID: the bridge uses ID 1, while the documented DPT146 factory address is 240.

### Corrected and validated DPT146 configuration (2026-08-28)

Direct polling confirmed this installed DPT146 was intentionally configured as slave ID 1 at `19200 8N2`. The original bridge table used one-based logical measurement addresses and `HH` word order. Those settings returned plausible but incorrect values because ENL-MOD-32 firmware 3.6 sends the entered address as the zero-based PDU address.

The bridge was corrected to PDU addresses `4, 6, 10, 20, 44, 512, 513, 515`. Vaisala 32-bit values use high byte first and low word first, represented as `HL` by firmware 3.6. The corrected table completed the bridge's detailed test with 8 successful reads and 0 exceptions. The clone-ready table is `artifacts/bridge-config/vaisala-dpt146-validated.tsv`.

### LoRaWAN state observed (2026-08-28)

The bridge is joined to a public North American hybrid 915 MHz LoRaWAN network. It uses a 15-minute transmit interval on uplink port 1, unconfirmed messages, ADR enabled, and displayed DR0/SF10/BW125 at 20 dBm. Optional bridge KPI values are excluded. Last observed receive metrics were -44 dBm RSSI and 27 dB SNR. Loriot receipt and raw-payload decoding are validated; downstream DSP routing is outside the present scope. See `artifacts/bridge-config/enl-mod-32-lorawan-status.md`.

A Loriot uplink on frame counter 18 validated the radio payload format. Firmware 3.6 encoded each configured point as a six-byte record: marker `0x10`, zero-based point index, and big-endian IEEE-754 float32 value. All eight DPT146 points decoded correctly. Existing infrastructure normally handles downstream routing after Loriot; DSP work is outside the present scope. See `artifacts/bridge-config/enl-mod-32-lorawan-payload.md`.

The bridge point-table reset and rollback procedure is also validated. Importing a point row with Slave ID `0` deletes that item. All eight points were deleted, the validated table was restored, all eight detailed reads succeeded with zero exceptions, and the restored configuration persisted across a bridge reboot. See `artifacts/bridge-config/enl-mod-32-config-reset-restore-validation.md`.

### Photo inventory

- `docs/evidence/bridge-overview.png` — Polygon ExactAire-E5, XP Power DRC30US24 supply, ETI EFD10 fuse holder, antenna, USB, and terminal blocks.
- `docs/evidence/usb-comi-switches.png` — USB-COMi-TB terminal and switch/jumper face.
- `docs/evidence/usb-comi-label.png` — USB-COMi-TB, serial number 620110022.
- `docs/evidence/dpt146-capabilities.png` — DPT146 label showing CH1 Td/f, CH2 pressure, RS485 Td/f/P/ppm/Td/f-atm/T, and 15–28 V input.
- `docs/evidence/dpt146-identity.png` — DPT146 B2MBD102A0X, serial number U2720970, manufactured in Finland in 2022.

Reference characteristics of the likely base hardware:

- 2-wire, half-duplex Modbus RTU over RS-485;
- 1,200–38,400 baud; 8 data bits; none/odd/even parity; 1 stop bit;
- 12–24 VDC input, 120 mA maximum; supply rated for at least 250 mA;
- local configuration over USB and remote configuration over LoRaWAN;
- RS-485 IN and OUT terminals internally daisy-chained;
- slave addresses 1–245, with 246 and 247 reserved by the bridge documentation;
- 150 ms maximum expected slave response latency;
- LoRaWAN Class A, with region determined by hardware/firmware.

## DPT146 read target

The Vaisala DPT146 uses Modbus RTU. Factory defaults, if they have not been changed, are:

- address `240`;
- `19200` baud;
- 8 data bits, even parity, 1 stop bit (`8E1`);
- function 03 for holding-register reads;
- wait at least five seconds after sensor power-up before polling.

### Critical connector warning for technicians

The Roman-numeral **connector labels I and II do not directly correspond to the CH1 and CH2 labels printed on the side of the DPT146**.

- **Connector I** is the analog-output connector. Its white and black conductors carry CH1 and CH2 analog signals.
- **Connector II** is the RS-485/Modbus connector. Its white and black conductors carry RS-485 D0- and D1+.
- The side label's `CH1` and `CH2` describe analog measurement outputs; they are not names for physical connector I and connector II.
- For the Polygon/Synetica Modbus bridge, the cable must be attached to **connector II**.

Both connectors use the same common 4-pin M8 A-coded form factor and the same conductor colors, so visual similarity makes this an easy installation mistake. Confirm the Roman numeral at the socket before troubleshooting addresses, baud rate, or polarity. Connecting the bridge wiring to connector I presents analog outputs to the RS-485 bus and produces timeouts or invalid serial bytes.

All measurement values are IEEE-754 32-bit floats stored as **least-significant word first, then most-significant word**. The table uses both the human-facing logical register and the zero-based PDU address actually placed in a Modbus request.

| Value | Logical registers | PDU start | Words | Unit | Access |
|---|---:|---:|---:|---|---|
| Temperature `T` | 5–6 | `0x0004` | 2 | °C | Read-only |
| Dew/frost point `Td/f` | 7–8 | `0x0006` | 2 | °C | Read-only |
| Atmospheric-pressure dew/frost point `Td/f atm` | 11–12 | `0x000A` | 2 | °C | Read-only |
| Moisture `H2O` | 21–22 | `0x0014` | 2 | ppmv | Read-only |
| Absolute pressure `P` | 45–46 | `0x002C` | 2 | bara | Read-only |
| Fault status | 513 | `0x0200` | 1 | 1 = no errors | Read-only |
| Online status | 514 | `0x0201` | 1 | 1 = data available | Read-only |
| Error code | 516–517 | `0x0203` | 2 | 32-bit flags | Read-only |

The instrument also exposes Modbus device identification: vendor, product code, firmware version, product name, serial number, calibration date, and calibration text.

Registers controlling purge and the Modbus address are deliberately excluded from the initial read plan. Vaisala warns that purge behavior is required for specified accuracy. The first application milestone is read-only.

## Documentation-needed assessment

### Blocking for reliable bridge configuration

- **Exact Polygon bridge identity:** photograph or transcription of every label, model number, serial number, FCC/IC ID, and regulatory label.
- **Polygon bridge user/configuration guide:** especially the meaning of “channel,” supported data types and word orders, register limit, polling interval, retry/timeout behavior, and clone/export format.
- **Configuration utility:** installer/version, supported Windows versions, and whether it talks to COM5 using a documented serial protocol or a proprietary one.
- **Current configuration export or screenshots:** capture before making any changes, including firmware version, Modbus serial settings, channel 1 definition, LoRa region, and reporting interval. Secrets such as AppKey/NwkKey must be redacted from project files and logs.
- **LoRaWAN integration contract:** network server/vendor, frequency plan (likely US915 for this location, but must be verified), activation method, DevEUI/JoinEUI, application payload decoder, uplink port, and expected DSP field schema.

### Blocking for a confirmed DPT146 live read

- DPT146 order code and serial number.
- Confirmation that connector II is the RS-485 connection in use and that the installed variant supports the desired temperature output.
- Current slave address, baud, parity, and stop bits. Factory defaults can be tried read-only but must not be assumed for production.
- RS-485 A/B/C terminal mapping from the actual cable installation; manufacturers sometimes use opposite A/B naming conventions.
- Whether the bridge supplies the DPT146 or it has an independent supply, plus measured supply voltage and common/reference wiring.
- A safe method to isolate the bridge before COM3 direct polling.

### Needed before each additional device profile is production-ready

- exact model and hardware/firmware revision;
- authoritative installation and Modbus manuals;
- complete register map, including function code, address convention, type, byte/word order, units, scaling, sentinel/error values, and writable-register hazards;
- factory and site serial settings;
- wiring/power requirements and termination/bias guidance;
- sample device or captured known-good responses;
- expected DSP names, units, precision, alarm limits, and reporting cadence;
- a signed-off validated configuration and acceptance test.

## Device backlog and documentation status

Most backlog devices are not presently available for bench testing. The standard approach is therefore to create documentation-backed, explicitly non-deployable `to-test` profiles now, using `artifacts/device-profiles/to-test/_template.yaml`. Hardware evidence later promotes a profile through `bench-validated`, `bridge-validated`, and finally `clone-ready`. The normative gates are documented in `artifacts/device-profiles/PROFILE-LIFECYCLE.md`.

### Initial scope

1. Vaisala DPT146 — validated reference.
2. Vaisala HMD65 (HMD60 family) — documentation-derived bridge test prepared; bench validation pending.
3. Continental Control Systems WattNode WND-M1-MB — documentation-derived bridge test prepared; bench validation pending.

All other previously listed devices remain deferred from active bench validation. Documentation-derived register tables are maintained for them so they can enter the profile lifecycle when hardware becomes available. The current configuration deliverable remains limited to tested or testable configurations for the three active devices.

The Advantech ADAM-4053 is outside the bridge target set. Its digital channels require coil/discrete I/O access, while the Synetica bridge is limited to function 3 and function 4 reads from holding and input registers.

The Lighthouse Solair 1100LD is also outside the bridge target set because its documented serial protocol is Modbus ASCII. The Synetica bridge supports Modbus RTU only.

### WattNode legacy-note assessment

The prior WattNode notes have been checked against the current manufacturer reference and preserved as an assessed device record in `docs/devices/wattnode-wnd-m1-mb-assessed-notes.md`. The key correction is that `CurrentIntScale` is a dimensionless full-scale integer count, not milliamps. With 250 A CTs and the default value 20000, the integer-current multiplier is 0.0125 A/count, not 0.01. Float measurement registers are preferred for the bridge POC because they avoid this integer scaling layer.

CT amp ratings are normal installation inputs. Gain and phase adjustments are calibration controls, while nominal CT voltage changes only for non-standard CT outputs; these settings must not be presented as routine guesses in the technician workflow.

### Hold for discussion before implementation

- Precision Digital PD2-6000.
- Lighthouse Solair 1100LD; the task dump currently includes an unrelated Precision Digital link that must be corrected.
- Seeed Studio SenseCAP S200 wind sensor.
- RKI VOC Pro.
- Advantech ADAM-4053; noted as unsupported by the Synetica bridge, so it may require a different acquisition path.

## Recommended product architecture

The normative three-layer design is documented in `docs/ARCHITECTURE.md`: device profiles, bridge adapters, and technician workflow. These layers must remain separate so additional instruments and bridge families do not require one-off GUI implementations.

Every clone-ready configuration must include a machine-readable manifest. Bridge model and firmware form its compatibility identity; imports are blocked on unvalidated firmware by default.

### Desktop application

Replace the prototype with one native Rust executable named `modbus-configurator.exe`. Use eframe/egui for the technician interface and an internal hardware-service thread as the single owner of COM ports, direct Modbus work, console navigation, backups, guarded writes, readback, and transcripts. Typed Rust commands and events cross that internal boundary; the same types serialize to JSON Lines for replay fixtures and diagnostics.

This migration addresses the observed bridge failure mode: a short PowerShell process used fixed delays and required an exact `Modbus Configuration Menu:` prompt even when the device could be in another valid state. PowerShell and the Python/Tk GUI remain useful as prototype references, but neither is part of the supported technician runtime.

Build a single release-mode Windows binary after bench validation. Keep profile and protocol logic outside the UI module so the state machines remain replay-testable and reusable.

Suggested layers:

1. **Workflow GUI** — guided commissioning, durable device states, plain-language failures, and expert diagnostics.
2. **Agent IPC** — versioned commands and events with request IDs, progress, results, cancellation, and stable error codes.
3. **Port actors** — one exclusive owner and operation queue per COM port so scanning cannot collide with a live action.
4. **Protocol adapters** — tolerant state machines for ENL-MOD-32 and native enLink consoles, plus direct Modbus RTU.
5. **Device profiles** — versioned YAML/JSON definitions for serial defaults, identity probes, measurements, register spans, decoding, units, and validation ranges.
6. **Bridge profile/compiler** — conversion of device measurements into the specific Polygon/enLink channel configuration and payload mapping.
7. **Evidence and export** — native backups, structured logs, configuration snapshots, test results, and shipment/DSP handoff reports.

The detailed internal boundary, state-machine rules, migration steps, and verification gates are in `docs/NATIVE-RUST-APPLICATION.md`.

### Profile concept

A profile should describe facts rather than contain GUI code. A DPT146 profile would identify the model, list candidate serial settings, define five measurement floats and three status values, specify LSW/MSW word order, and provide plausible ranges. Adding a new instrument should normally mean adding and validating a profile rather than editing screens.

Profiles need versioning and provenance fields: manufacturer, model, applicable firmware, source manual/document revision, reviewer, test date, and test-device serial number.

## Technician workflow

The intended operator experience is a guarded wizard:

1. **Connect** — show illustrated wiring and power checks; detect COM3/COM5 automatically.
2. **Snapshot** — read and save the bridge's current configuration before edits.
3. **Select or identify instrument** — auto-probe only safe identity/read registers; otherwise select a model from a searchable list.
4. **Verify serial link** — test address/baud/parity from the profile using conservative, read-only requests.
5. **Preview measurements** — show live engineering values, quality/status, raw registers, and timestamps.
6. **Choose outputs** — default to the recommended measurement set, with expert overrides.
7. **Compile bridge channels** — display exactly what registers and decoders will be programmed.
8. **Validate** — perform local readback, LoRa uplink check, and DSP-field comparison.
9. **Commit and label** — write only after explicit confirmation; export configuration and a shipment acceptance report.

The normal mode should hide register arithmetic. Expert mode should expose raw request/response frames, zero-based versus one-based address interpretation, byte/word order, and timeouts.

## Delivery plan

### Phase 0 — Preserve and identify

- Photograph labels and wiring.
- Acquire the Polygon software/manual and capture the current bridge configuration.
- Confirm the LoRaWAN/DSP path and payload decoder.
- Record DPT146 identity and current serial parameters.

**Exit:** enough evidence to restore the existing system after any experiment.

### Phase 1 — Read-only DPT146 bench tool

- Isolate the bridge from the bus.
- Poll the DPT146 through COM3.
- Read device identification, all five measurements, and all three status values.
- Confirm float word order and units against plausible physical readings.
- Save raw frames and a timestamped result.

**Exit:** repeatable, non-destructive direct read of every supported DPT146 value.

### Phase 2 — Profile engine and basic GUI

- Encode the DPT146 as the first versioned device profile.
- Build port discovery, connection settings, live-value table, error messages, and diagnostic logging.
- Add a simulator/replay transport so UI development and technician training do not require hardware.

**Exit:** a technician can connect and validate a DPT146 without entering register numbers.

### Phase 3 — Bridge integration

- Preserve the validated configuration tables and captured bridge behavior as replay fixtures.
- Build the native Rust GUI, persistent hardware service, and typed command/event interface.
- Implement state-aware login, menu recovery, read/export, and Read All through the agent.
- Validate read-only operations on the physical bridge before exposing writes.
- Add writes only with native backup, diff, confirmation, exported readback, and recovery evidence.
- Validate the LoRa uplink and raw-payload decoding in Loriot.

**Exit:** one-click deployment of a known-good DPT146 channel set with proof of correctly decoded Loriot values.

### Phase 4 — Shipment intake system

- Bench-validate the prepared WND-M1-MB and HMD65 profiles when physical units become available.
- Create guided intake checks, validated fixtures, acceptance criteria, and printable/exportable reports.
- Integrate the profile/compiler library into the auto-programmer system if its ownership and interface are confirmed.

**Exit:** a repeatable shipment-to-Loriot workflow that can grow one reviewed device profile at a time; downstream DSP routing remains a separate concern.

## Current handoff

The complete dated handoff is `CURRENT-STATUS.md`. The bridge currently contains the restored DPT146 validated table, reports 8 configured points with 8 successful reads and 0 exceptions, and retained that state across reboot.

The bridge can be disconnected while the agent contract, parsers, replay fixtures, and GUI integration are built. Bring it back for each read-only state-machine gate and again before any write path is exposed. HMD65 and WND-M1-MB promotion still requires their physical hardware.

## Sources

- [Vaisala DPT146 User's Guide M211372EN-E](https://docs.vaisala.com/v/u/M211372EN-E/en-US)
- [Synetica enLink LoRaWAN Modbus RS485 Master Hardware Guide SYN-ENL-0101F](https://synetica.net/wp-content/uploads/2019/04/enLink-Modbus-RS485-Hardware-Guide-SYN-ENL-0101F.pdf)
- [Synetica enLink Modbus product brochure](https://www.synetica.net/wp-content/uploads/2023/04/enLink-Modbus-Brochure-1.pdf)
- [Continental Control Systems WattNode Modbus support](https://ctlsys.com/support/wattnode-module-modbus/)
- [Vaisala HMD60/HMD65 product documentation](https://www.vaisala.com/en/products/instruments-sensors-and-other-measurement-devices/instruments-industrial-measurements/hmd60)
