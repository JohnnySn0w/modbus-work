# Configurator architecture

The configurator is intentionally divided into three reusable domain layers. Device knowledge, bridge behavior, and technician workflow must remain separate so new instruments and new bridge families can be added without duplicating screens or protocol logic.

## Runtime topology

The supported Windows runtime is being separated into two processes:

- the existing Python/Tk application remains the technician-facing presentation layer during migration;
- a long-running Rust executable, `modbus-agent.exe`, becomes the sole owner of serial ports, protocol state machines, direct Modbus reads, bridge configuration, native-device configuration, and hardware transcripts.

The two processes initially communicate through versioned newline-delimited JSON over standard input and output. This keeps the GUI replaceable, makes hardware conversations replay-testable, and prevents the background scanner and explicit device actions from competing for a COM port. PowerShell console automation is retained only as a temporary bench diagnostic path and is not the target production transport.

See [RUST-HARDWARE-AGENT.md](RUST-HARDWARE-AGENT.md) for the IPC, state-machine, migration, safety, and verification design.

## 1. Device profiles

A device profile describes the Modbus instrument independently of any bridge or GUI.

Required profile content:

- manufacturer, product family, exact model, and applicable firmware/hardware revisions;
- identity probes and safe detection rules;
- supported transports and serial defaults/candidates;
- slave-address rules;
- register type, function code, zero-based PDU address, and manual/logical address;
- data type, byte order, word order, scaling, unit, precision, and valid range;
- measurement name, stable machine key, display label, and DSP semantic name;
- status/error interpretation and unavailable/sentinel values;
- read/write safety classification;
- wiring, power, termination, connector, and commissioning notes;
- source-manual document code/revision and tested-device evidence.

The DPT146 is the first validated reference profile. Documentation-derived, non-deployable test profiles and ENL-MOD-32 candidate tables are prepared for the HMD65 and the specific WND-M1-MB WattNode Module for Modbus. All other device families are currently deferred.

Device profiles follow the lifecycle defined in `artifacts/device-profiles/PROFILE-LIFECYCLE.md`. Devices without available hardware receive documentation-backed `to-test` profiles with deployment blocked and all unknowns explicit. Physical evidence is required for promotion to bench-validated, bridge-validated, and clone-ready status.

## 2. Bridge adapters

A bridge adapter converts selected points from one or more device profiles into a bridge-specific configuration and transport payload.

Required adapter capabilities:

- identify bridge model, firmware, and configuration protocol;
- treat model plus firmware version as the bridge adapter compatibility identity;
- read/export and restore the existing configuration;
- represent bridge-wide serial, polling, retry, timeout, and reporting settings;
- compile device-profile measurements into bridge data-point slots;
- enforce capacity and compatibility constraints;
- import/write, read back, diff, validate, and roll back;
- describe LoRaWAN region, activation, join state, reporting interval, payload format, and decoder contract;
- keep network/application security keys out of routine logs and redacted evidence; when explicitly required for this private project, store them only in clearly named private clone/recovery artifacts;
- generate a clone-ready bridge artifact and a human-readable configuration summary.
- ship a machine-readable manifest with every approved configuration, including tested firmware, schema version, source profiles, validation state, and compatibility policy.

The first adapter target is Polygon ExactAire-E5 / Synetica ENL-MOD-32 firmware 3.6, which supports 32 Modbus data points.

Approved configurations default to exact-firmware compatibility. A GUI must block import on an unvalidated firmware version unless a reviewed compatibility rule explicitly allows it. Firmware changes can affect menu protocol, address interpretation, word-order codes, point capacity, import/export format, and LoRaWAN payload encoding.

## 2a. Native-device adapters

Direct LoRaWAN products such as the Synetica enLink IAQ Plus bypass the Modbus bridge but reuse the same profile and technician workflow concepts. A native-device adapter owns USB/banner identification, authenticated console navigation, region compatibility, configuration backup, live-value parsing, field-level writes, readback, and reboot behavior. It must not place console-menu knowledge in GUI screens.

The first native adapter targets `FW-AQ-VCP+` 5.06 on Windows. Background discovery stops at the unauthenticated banner. Explicit technician actions may derive the login from the DevEUI and read configuration pages. Region and other writes remain blocked until exact prompts and recovery behavior are validated.

## 3. Technician workflow

The technician-facing application is primarily a guarded auto-configuration tool, not merely a monitoring dashboard. It orchestrates profiles and adapters through a plug-in, select, program, and prove commissioning sequence:

1. detect laptop interfaces and connected bridge hardware;
2. show model-specific wiring and connector checks;
3. identify the connected instrument where safe, or let the technician select an approved pre-made profile;
4. back up the bridge before any write;
5. verify direct, read-only Modbus communications;
6. preview engineering values and status;
7. select required device measurements and preview the complete programming diff;
8. compile and display the bridge configuration diff;
9. program supported device settings and import the bridge table only after confirmation;
10. read back and require local validation;
11. verify LoRaWAN join and uplink;
12. validate raw payload decoding in Loriot;
13. export the clone package and shipment acceptance report.

For the initial Modbus device POC phase, step 12 ends at validated Loriot raw-payload decoding. Existing infrastructure normally handles downstream DSP routing after Loriot; that routing is outside this project's present scope and will be revisited only after multiple device profiles and the generic configuration model have been proven.

Normal mode should avoid register arithmetic and protocol jargon. Expert mode should expose raw frames, PDU versus manual addresses, decoding order, timing, and diagnostic logs.

The product succeeds when an operations technician can connect known hardware, select an approved configuration, safely program it, and prove live engineering values without interpreting Modbus registers. DSP handoff is the final downstream documentation section after local and Loriot validation.

## Core rule

No device-specific register logic belongs in GUI screens, and no bridge-specific import logic belongs in device profiles. The workflow layer composes the two through a stable internal model. Only the hardware agent owns live COM ports; the GUI consumes structured events and results rather than console text or PowerShell exceptions.
