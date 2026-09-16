# Operator guide

## Start and connect

Extract the current Windows release archive (.zip) and run Polygon Device Configurator.exe. Close other instances before starting a replacement. The app automatically discovers routes and polls supported devices; Serial port numbers may change. Verify the physical model and wiring independently of any selected E5 bridge profile.

For direct USB adapter reads on the shared bus, switch the E5 bridge off using its hardware switch. External power may remain; its USB interface disappears when switched off. Adapter requests are blocked while an E5 bridge interface is detected. Only one Modbus master may drive the bus.

## Readings

Devices shows current or retained data. Read now requests a manual read. Last-good timestamps and stale labels identify old data after failures/disconnection; zero is retained when it is a genuine reading. The adapter detail view shows readings received through that connection. A profile match does not verify the physical sensor model. Register tables wrap long descriptions and offer unit cycling and native/display comparison where applicable. Unit changes do not rewrite sensor registers or E5 bridge tables.

## Configure an E5 bridge

1. Open Configuration and wait for a verified target. An orange banner identifies an unavailable target; hardware actions stay disabled.
2. Select a sensor profile, Open configuration file, or a saved backup. Selection alone does not write.
3. Review changed/removed/added points and communication requirements. Edit Sensor slave address to match the physical sensor. Candidate profiles are not evidence of physical qualification. Use the separate RS-485 line settings editor for downstream baud, parity and framing; tab-separated configuration files (.tsv) do not contain those settings. See [E5 line configuration](E5-LINE-CONFIGURATION.md).
4. Program E5 bridge queues behind an active automatic read. The application reads the identity and current table again, requires a successfully saved backup, writes with acknowledgements and verifies exported contents.
5. Inspect fresh readings after completion. On an interrupted or uncertain write, recover/check the console and review the saved backup before a restore. The application does not automatically restore an earlier configuration.

**Back up E5 bridge** retrieves the current point table. Enter an optional **Backup name** to keep a named copy alongside the automatic snapshot. Reusing a name creates another version without overwriting an earlier backup. **Load backup** lists versions associated with the USB connection identity by name, or provides selection of a tab-separated configuration file (.tsv). Loading is for review; use **Program E5 bridge** to restore it. Save configuration file and Copy configuration table use the current selection. Point-table backups exclude radio keys, serial framing, sensor calibration and firmware.

## History, settings and help

The HMD65 E5 bridge profiles use one-based manual register numbers and exclude the error-code pair at 514–515. Each contains 11 entries. This convention applies to E5 bridge configuration files; direct Modbus requests and manufacturer reference addresses remain zero-based. Existing backups are preserved without conversion.

History contains up to 50,000 point samples from this session. Choose the measurement/route; axes show native value units and elapsed seconds. Failures leave gaps. Export a comma-separated values file (.csv) before closing if the history is needed later.

Settings: System/Light/Dark theme; System/United States/United Kingdom/Europe unit defaults; automatic polling. System units select the United States preset for that Windows region, the United Kingdom preset for that region, and the Europe preset for other or unavailable regions. The United States preset uses degrees Fahrenheit and pounds per square inch absolute where applicable. The United Kingdom preset uses degrees Celsius and bar; Europe uses degrees Celsius and kilopascals. Explicit register overrides last for the session. Polling off stops future automatic requests after the current operation finishes; it does not stop the E5 bridge from requesting readings on its serial bus.

References contains product information, register maps and Device manuals PDF buttons. Unavailable manuals are marked. Troubleshooting provides application guidance and device-specific material; instructions without established support show TBD. Diagnostics contains route and activity details; successful automatic polling is omitted from the activity log.

Selecting **Use this interface** in Diagnostics turns off automatic polling for the current session. Manual actions remain available. Enable **Automatic polling** in Settings to resume automatic readings. Selecting an interface does not change the saved startup preference.

**Activity log** retains the latest 64 events for the session. Each entry includes a local date and time. Serial operation entries include the request number and interface, with progress, results, backup paths, line settings, and any register errors. Connection changes and automatic polling failures are recorded; successful automatic polling is omitted. **Copy log** reports whether the system clipboard write succeeded. **Save log…** exports the timestamped entries to a text file. Closing the application discards activity that has not been copied or saved.

**Export diagnostics…** creates one text file for support from persistent logs under `%LOCALAPPDATA%\Polygon\Device Configurator\Logs`. These include build identification, background operation summaries, errors, and Rust panic backtraces. Up to ten recent process logs are retained, each with a 5 MiB rotation threshold and one previous file. Clearing the visible activity list does not delete these files. Logs can contain device identifiers and local file paths; review the export before sharing. Native crashes, forced termination, and power loss do not guarantee a final log entry or a crash dump.

Settings.json, Selected.tsv and Backups are stored beneath `%LOCALAPPDATA%/Polygon/Device Configurator/`. History itself is not persisted. Avoid sharing logs or configuration material that includes credentials.

Firmware updating is not available yet. See [firmware plan](FIRMWARE-UPDATES.md).

## Basis of the guidance

E5 bridge and Vaisala DPT146 instructions use recorded test results. Other device instructions are either explicitly attributed to manufacturer documentation or marked **TBD**. Documentation-based suggestions are not hardware verification. Historical notes and proposed test plans are not approved installation procedures.
