# Modbus bridge configurator

This repository contains the device profiles, bridge configurations, validation evidence, and technician-facing auto-configuration GUI for Polygon ExactAire-E5 / Synetica ENL-MOD-32 LoRaWAN Modbus bridges.

The intended operator experience is: **plug in the hardware, select an approved pre-made configuration, let the tool program it safely, and confirm real data before the unit leaves the bench.**

The current Modbus scope is intentionally limited to three instruments, plus one directly connected Synetica LoRaWAN sensor used to extend the same technician workflow:

- **Vaisala DPT146** — validated configuration, tested locally through Modbus, through the bridge, and as a decoded Loriot uplink.
- **Vaisala HMD65** — documentation-derived test configuration awaiting physical hardware.
- **Continental Control Systems WND-M1-MB** — documentation-derived test configuration for the WattNode Module for Modbus, awaiting physical hardware.
- **Synetica enLink IAQ Plus, observed part 003-ADZ-301** — identified live through its USB console as `FW-AQ-VCP+` firmware 5.06 on North American Hybrid FSB #1 / 915 MHz.

Start with [CURRENT-STATUS.md](CURRENT-STATUS.md) for the complete bench handoff and current state.

## Architectural strokes

The system is split into three layers so new devices and future bridge firmware do not require one-off screens:

1. **Device profiles** describe an instrument independently of the bridge: identity, serial settings, safe detection, registers, decoding, units, status values, installation inputs, and write hazards.
2. **Bridge adapters** compile selected profile points into a firmware-specific bridge configuration. The ENL-MOD-32 adapter owns its eight-column TSV format, address behavior, word-order codes, capacity, backup, delete, import, readback, and rollback rules.
3. **Technician workflow** guides discovery, backup, device selection, read-only validation, configuration preview, guarded write/import, readback, Loriot validation, and export.

The important boundary is that device-specific register logic does not live in GUI screens, and bridge import behavior does not live in device profiles. See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and [the profile lifecycle](artifacts/device-profiles/PROFILE-LIFECYCLE.md).

## Component details

### Bridge

The validated bridge is a Polygon ExactAire-E5 white-label of the Synetica ENL-MOD-32, firmware 3.6. It supports 32 Modbus points and is configured through an STM32 USB virtual COM interface. Its tested bus settings are 19200 baud, 8 data bits, no parity, and 2 stop bits.

Firmware 3.6 accepts tab-delimited rows with these columns:

```text
Item  ID  Reg  Addr  Data  Word  Mult  Read
```

Entering Slave ID `0` deletes an item. That deletion method, golden restoration, detailed readback, and reboot persistence have all been validated.

### DPT146

The DPT146's side-label `CH1` and `CH2` are analog channels. They do not correspond directly to physical connectors I and II. **Connector II is the RS-485/Modbus port.** The installed unit uses slave 1 at 19200 8N2. Its 32-bit values use low-word-first order, represented as `HL` by bridge firmware 3.6.

### HMD65

The prepared test profile contains eight metric float measurements and four status points. Address, protocol, bitrate, parity, and termination come from physical DIP switches. Float word order must be confirmed on hardware; `HH` is the first candidate and `HL` is the controlled fallback.

### WND-M1-MB

The target is specifically the **WND-M1-MB WattNode Module for Modbus**, not the Wide-Range meter. The candidate uses 12 native float measurements so current and power do not need integer scaling. CT ratings and electrical service mapping remain installation inputs.

The physical bench gate and evidence sequence are in [docs/WATTNODE-TEST-READINESS.md](docs/WATTNODE-TEST-READINESS.md). It includes bounded first-contact settings, exact identity/config snapshot registers, CT and service mapping decisions, and bridge/Loriot acceptance steps.

### enLink IAQ Plus

The IAQ Plus is a direct LoRaWAN sensor, not a Modbus instrument behind the bridge. Its unauthenticated USB banner provides a deterministic product-family signature, firmware code/version, radio region, and DevEUI. This is necessary because it shares the STM32 `0483:5740` USB identity with other Synetica products. Its serial login is deterministically derived by normalizing the displayed DevEUI and taking its final four hexadecimal characters; the observed `0004a30b00084f86` therefore yields `4f86`. Background discovery never submits credentials. The application defaults to US915 Hybrid FSB #1. An EU868 profile exists but remains disabled until compatible regional hardware/firmware and Loriot settings are supplied and validated.

Firmware `FW-AQ-VCP+` 5.06 is supported by a Windows read-only console prototype. The GUI can refresh installed sensor readings, create a JSON backup of Quick Start/radio/configuration pages, and preview the enabled US915 or disabled EU868 radio profiles. The observed unit exposes temperature, relative humidity, pressure, CO₂-equivalent, bVOC, IAQ/accuracy, and a GSS CO₂ module. Device writes remain disabled until field-level readback and recovery are proven.

The first approved-target draft is [the US915 IAQ Plus native configuration](artifacts/native-config/synetica-enlink-iaq-plus-us915.yaml). IAQ commissioning preserves the existing JoinEUI/AppEUI and provisions only a selected AppKey; measurement display is optional verification. The attached IAQ JoinEUI already matches the project value, but its current AppKey differs from the stored bridge/project AppKey, so the target credential profile remains an explicit commissioning choice.

## GUI

The desktop GUI uses Python's built-in Tk toolkit and keeps serial discovery separate from device definitions.

Features currently implemented:

- automatic serial-port discovery every 1.5 seconds; COM numbers are routes rather than identities, USB metadata selects candidate transports, and protocol/banner fingerprints identify products;
- recognition of the known ENL-MOD-32 USB interface and USB-COMi-TB bench adapter;
- deterministic, read-only Modbus fingerprints when the USB-COMi-TB is the only master present: DPT146 measurement/status layout, HMD65 eight-value/status layout, and WattNode identity plus diagnostic model/firmware/serial registers;
- a connected-device diagram showing the serial interface and attached/configured instrument;
- clean, clickable device cards with generated device illustrations;
- detail pages with a top-left back arrow, connection information, readouts, device facts, and links to source artifacts;
- register-table pages for every supported Modbus instrument, showing manual and zero-based PDU addresses, data type, access, live/saved readout, decoded enum/bitfield meaning, units, and plain-language descriptions;
- HMD65 and WND-M1-MB cards clearly marked as ready-to-test rather than connected or validated;
- automatic refresh every 1.5 seconds plus a manual refresh action;
- one-click creation of a portable configuration backup ZIP containing the golden table, manifest, bridge settings, radio notes, and recovery credential record;
- a basic pre-made configuration picker for DPT146, HMD65, and WND-M1-MB that copies a TSV for review without writing to hardware;
- per-device Help dialogs containing short setup and troubleshooting guidance.
- IAQ Plus authenticated live-reading refresh, private JSON console backup, and radio-profile preview;
- firmware-package preflight with exact identity/region/upgrade-path checks and SHA-256 validation; actual flashing remains blocked pending vendor tooling and recovery instructions;

When a direct fingerprint succeeds, the detail page displays the live values returned by that probe. Otherwise, DPT146 values are explicitly labeled as the latest validated bench readings. The transport and fingerprint layer owns register logic; GUI screens do not.

Active fingerprinting is deliberately gated. It runs only when the direct USB-COMi-TB adapter is present and the bridge USB interface is absent, preventing the tool from becoming a second Modbus master on the bridge bus. A signature requires the documented serial format, slave response, register map, data encoding, status layout, and plausible decoded values to agree. A WattNode family-only identity is shown as such and requires a WND-M1-MB label check rather than being presented as an exact-model match.

USB interface presence is refreshed every 1.5 seconds. Cached identities are discarded when an interface disappears, so reconnecting the same device on the same COM number triggers identification again. Because USB presence alone cannot establish whether another device is acting as the RS-485 master, active Modbus transmission is separate: discovery performs only one bounded direct fingerprint attempt per adapter connection or manual Refresh. A failed scan leaves the adapter highlighted yellow and explains that either nothing is connected or another master may be active; it does not identify that master or report the downstream instrument as absent.

## Target technician workflow

The GUI is evolving into a guarded auto-configuration tool. Its normal workflow will be:

1. **Plug in** — detect the bridge USB interface, direct RS-485 adapter, and any safely identifiable instrument.
2. **Inspect** — show the connected-device diagram, exact ports, model/firmware information, and wiring help.
3. **Choose** — select an approved pre-made device configuration. Normal users should not enter register numbers.
4. **Back up** — capture bridge settings, point table, firmware identity, LoRaWAN recovery values, and readable device configuration.
5. **Preflight** — compare the selected profile with bridge firmware, device identity, serial settings, point capacity, and installation inputs.
6. **Preview** — show proposed changes in plain language, with an expert view for raw details.
7. **Program** — apply supported device settings and import the bridge table only after confirmation.
8. **Read back** — reread the programmed state and require a reviewed match.
9. **Confirm data** — display live engineering values, status, and plausible-range checks.
10. **Validate Loriot** — confirm join, uplink, and correctly decoded raw payload.
11. **Export** — produce the clone package and a simple commissioning record.

Writes remain profile-driven and safety-classified. Installation settings may be offered through guarded forms; calibration controls remain expert-only; destructive or undocumented writes remain blocked.

### Run on Windows

Install Python 3.11 or newer, then from the repository root:

```powershell
python -m pip install -r requirements.txt
.\run_gui.ps1
```

Or run directly:

```powershell
python -m app.main
```

`pyserial` improves hardware names and VID/PID matching. The app has a Windows registry fallback and will still start without it.

For a command-line IAQ Plus backup on Windows:

```powershell
.\.venv\Scripts\python.exe .\tools\enlink_snapshot.py --port COM5 --output .\.secrets\enlink-iaq-plus-console-backup.json
```

The backup intentionally contains LoRaWAN credentials. Store it only in the private repository or an approved commissioning location.

Firmware update design and current vendor-material blockers are documented in [Firmware updates](docs/FIRMWARE-UPDATES.md). The GUI can inspect a versioned package manifest today, but it will not flash an image until the vendor method and recovery path are bound and tested.

## Repository map

```text
app/                         GUI, device catalog, and hardware discovery
artifacts/bridge-config/     Golden and documentation-derived bridge tables
artifacts/device-profiles/   Profile lifecycle and machine-readable profiles
artifacts/native-config/     Native LoRaWAN device target configurations
docs/evidence/               Bench photographs and screenshots
docs/reference/              Manufacturer manuals
tools/                       Low-level bridge console and credential utilities
CURRENT-STATUS.md            Current handoff and next work
PROJECT.md                   Full project record
```

## Safety

- Never attach the USB-COMi-TB as an active second master to the energized bridge bus.
- Back up the bridge before writes.
- Show a configuration diff before import.
- Require readback after import.
- Treat calibration registers as expert-only operations.
- Do not promote documentation-only profiles until physical hardware and Loriot payloads are validated.

## DSP how-to

This will be the final stage of the technician documentation after the Modbus auto-configuration workflow is complete and proven across the three active devices.

Planned contents:

1. Locate the commissioned device in Loriot by DevEUI.
2. Confirm uplink port, interval, frame counter progression, and payload length.
3. Apply or verify the firmware-specific payload decoder.
4. Confirm that every point index maps to the intended measurement and unit.
5. Follow the organization's existing Loriot-to-DSP routing procedure.
6. Verify destination field names, units, precision, timestamps, and alarm semantics.
7. Save evidence in the commissioning package.

DSP routing is not implemented by this repository today. This section remains deliberately last so bridge programming and live Modbus/Loriot validation do not require DSP-domain access.
