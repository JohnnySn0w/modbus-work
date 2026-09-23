# Operator guide

## Start and connect

Extract the current Windows release archive (.zip) and run Polygon Device Configurator.exe. Close other instances before starting a replacement. The app automatically discovers routes and polls supported devices; Serial port numbers may change. Verify the physical model and wiring independently of any selected Modbus Bridge profile.

For direct USB adapter reads on the shared bus, switch the Modbus Bridge off using its hardware switch. External power may remain; its USB interface disappears when switched off. Adapter requests are blocked while a Modbus Bridge interface is detected. Only one Modbus master may drive the bus.

## Readings

Devices shows current or retained data. Read now requests a manual read. Last-good timestamps and stale labels identify old data after failures/disconnection; zero is retained when it is a genuine reading. The adapter detail view shows readings received through that connection. A profile match does not verify the physical sensor model. Register tables wrap long descriptions and offer unit cycling and native/display comparison where applicable. Unit changes do not rewrite sensor registers or Modbus Bridge tables.

## Configure a Modbus Bridge

1. Open Configuration and wait for a verified target. An orange banner identifies an unavailable target; hardware actions stay disabled.
2. Configuration starts with one device entry. Choose its model and slave address, then use **Add device** for more sensors (up to 32 devices and 32 register entries total). There is no separate single-device mode. You can also open a configuration file or saved backup: each slave becomes an editable entry, with unmatched rows preserved as a custom register set. Selection alone does not write.
3. Review changed/removed/added points and communication requirements. Edit Sensor slave address to match the physical sensor. Candidate profiles are not evidence of physical qualification. Use the separate RS-485 line settings editor for downstream baud, parity and framing; tab-separated configuration files (.tsv) do not contain those settings. See [Modbus Bridge line configuration](E5-LINE-CONFIGURATION.md).
4. Program Modbus Bridge queues behind an active automatic read. The application reads the identity and current table again, requires a successfully saved backup, writes with acknowledgements and verifies exported contents.
5. Inspect fresh readings after completion. On an interrupted or uncertain write, recover/check the console and review the saved backup before a restore. The application does not automatically restore an earlier configuration.

**Back up Modbus Bridge** retrieves the current point table. Enter an optional **Backup name** to keep a named copy alongside the automatic snapshot. Reusing a name creates another version without overwriting an earlier backup. **Load backup** lists versions associated with the USB connection identity by name, or provides selection of a tab-separated configuration file (.tsv). Loading is for review; use **Program Modbus Bridge** to restore it. Save configuration file and Copy configuration table use the current selection. Point-table backups exclude radio keys, serial framing, sensor calibration and firmware.

## History, settings and help

The HMD65 Modbus Bridge profiles use one-based manual register numbers and exclude registers 514–515, 518 and 519. Each contains eight entries, including device status at 513. Reprogram an existing bridge with the updated profile to remove these entries from its reads. This convention applies to Modbus Bridge configuration files; direct Modbus requests and manufacturer reference addresses remain zero-based. Existing backups are preserved without conversion.

History contains up to 50,000 point samples from this session. Each recorded register has its own graph, labeled by slave address and source. Different register settings remain separate. Axes show native value units and elapsed seconds. Failures leave gaps. Export a comma-separated values file (.csv) before closing if the history is needed later.

Settings: System/Light/Dark theme; System/United States/United Kingdom/Europe unit defaults; automatic polling. System units select the United States preset for that Windows region, the United Kingdom preset for that region, and the Europe preset for other or unavailable regions. The United States preset uses degrees Fahrenheit and pounds per square inch absolute where applicable. The United Kingdom preset uses degrees Celsius and bar; Europe uses degrees Celsius and kilopascals. Explicit register overrides last for the session. Polling off stops future automatic requests after the current operation finishes; it does not stop the Modbus Bridge from requesting readings on its serial bus.

References contains product information, register maps and Device manuals PDF buttons. Unavailable manuals are marked. Troubleshooting provides application guidance and device-specific material; instructions without established support show TBD. Diagnostics contains route and activity details; successful automatic polling is omitted from the activity log.

Selecting **Use this interface** in Diagnostics turns off automatic polling for the current session. Manual actions remain available. Enable **Automatic polling** in Settings to resume automatic readings. Selecting an interface does not change the saved startup preference.

**Activity log** retains the latest 64 events for the session. Each entry includes a local date and time. Serial operation entries include the request number and interface, with progress, results, backup paths, line settings, and any register errors. Connection changes and automatic polling failures are recorded; successful automatic polling is omitted. **Copy log** reports whether the system clipboard write succeeded. **Save log…** exports the timestamped entries to a text file. Closing the application discards activity that has not been copied or saved.

**Export diagnostics…** creates one text file for support from persistent logs under `%LOCALAPPDATA%\Polygon\Device Configurator\Logs`. These include build identification, background operation summaries, errors, and Rust panic backtraces. Up to ten recent process logs are retained, each with a 5 MiB rotation threshold and one previous file. Clearing the visible activity list does not delete these files. Logs can contain device identifiers and local file paths; review the export before sharing. Native crashes, forced termination, and power loss do not guarantee a final log entry or a crash dump.

**Communication and data checks** assesses the last response before retained values are added to the display. Each snapshot names its interface and timestamp; it is not a live connection indicator. Checks compare each slave's full or partial register layout against reviewed profiles. A consistent address difference of one is reported as a possible indexing mismatch, never automatically corrected. The reviewed HMD65 Modbus Bridge convention is preserved.

Assessment findings also appear in Activity log after manual reads or when automatic findings change. **Copy log** and **Save log…** include the complete latest assessment even if the visible activity list has been cleared or its oldest entries trimmed. **Export diagnostics…** includes assessment history across retained sessions.

Value checks flag non-finite numbers, broad physical bounds (such as relative humidity outside 0–100%), suspicious subnormal floating-point values, and defined fault/status values. These are plausibility checks, not accuracy checks or complete manufacturer operating limits. Zero alone is valid. Unknown or ambiguous profiles cannot receive device-specific interpretation. Incorrect indexing and word order can still produce plausible values.

Serial checks show the reported Modbus Bridge settings, flag incompatible data-bit settings, and compare the timeout against an estimated request/response transmission time. Instrument baud rate and parity cannot be inferred from a timeout or from a successful USB console connection. No automatic baud scan, alternate-address reads, or configuration writes are performed. Findings are included in exported diagnostics.

**Settings → Advanced communication settings** controls the application's waits, independently of the Modbus Bridge's sensor settings. Defaults are 20 seconds for the initial console response, 5 seconds for menu/command replies, 180 seconds minimum for each Read All phase, and a 30-second retry delay for recoverable errors. Console timeouts and transport failures pause automatic polling. Automatic scan budgeting is enabled by default: it allows at least 15 seconds per configured entry plus 30 seconds, or more for longer reported sensor retries, capped at one hour per phase. Disable **Allow extra scan time for sensor retries** to test the exact configured Read All deadline. Changes apply at the next operation without reopening the active connection. A silent initial connection can consume an initial wait plus one wake-response wait. Read All can have two response phases. Longer limits allow more waiting but do not establish that a scan is progressing. Reset communication defaults restores these values.

The Devices map displays the verified configuration before a sensor scan finishes. Cards retain configured order and show slave-address badges. Reported point errors are attributed to their slave immediately; a global console timeout alone cannot identify which sensor failed. Configuration remains available after read failure, with **Check recovered console** to export the table without starting another scan. An interrupted write still requires recovery and review.

For a connection failure, turn automatic polling off, use **Read configuration only** in Diagnostics to check console access without polling instruments, then use **Read now** for the sensor read. **Live communication timeline** shows response deadlines, first-byte latency, byte counts, last-byte age, parser state, and bounded receive-recovery attempts. Read All also records configured entry count, slave addresses, and downstream settings when available. A successful recovery call is not proof that data resumed. No passwords or response payloads are recorded. **Copy log**, **Save log…**, and **Export diagnostics…** include the timeline. Use Export diagnostics for the longer retained history.

Settings.json, Selected.tsv and Backups are stored beneath `%LOCALAPPDATA%/Polygon/Device Configurator/`. History itself is not persisted. Avoid sharing logs or configuration material that includes credentials.

Firmware updating is not available yet. See [firmware plan](FIRMWARE-UPDATES.md).

## Basis of the guidance

Modbus Bridge and Vaisala DPT146 instructions use recorded test results. Other device instructions are either explicitly attributed to manufacturer documentation or marked **TBD**. Documentation-based suggestions are not hardware verification. Historical notes and proposed test plans are not approved installation procedures.

Enthalpy is excluded from both HMD65 profiles: registers 15–16 (metric) and 143–144 (non-metric). Adapter discovery reads only the seven metric measurements and device status at 513; it does not read the excluded status registers.

Device details include every received register value. Live register tables highlight successful readings and show seconds since each register last succeeded; failed reads keep the prior value and increasing age. Registers without a successful reading have no age. When all entries for one slave fail, check power, wiring and serial settings as well as its register configuration.

Use **Clear errors** beside **Refresh** to acknowledge displayed warnings and reset consecutive failure counts. This preserves diagnostic logs, last-good readings and safety blocks, and does not write to the instruments. New errors appear again when reported.

The Modbus Bridge card, device details and Configuration heading show its LoRa EUI after the console banner is read. **Copy EUI** copies exactly 16 uppercase hexadecimal characters, without spaces or separators. This identifier is separate from the USB serial number. It remains unavailable until the current bridge supplies a valid identifier.

Selecting a device type for a custom register set opens the normal device summary with its image and reading cards. **Register table** opens the detailed readings for that slave. Units apply only to entries matching the selected model's register encoding.
