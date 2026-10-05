# Operator guide

Use this guide for daily operation. For detailed checks, time limits and log export, see [diagnostics](DIAGNOSTICS.md). [Device support](../CURRENT-STATUS.md) distinguishes available functions from recorded hardware verification.

## Start and connect

Extract the current Windows release archive (.zip) and run Polygon Device Configurator.exe. Close other instances before starting a replacement. The app automatically discovers routes and polls supported devices; serial port numbers may change. Verify the physical model and wiring independently of any selected Modbus Bridge profile.

For direct USB adapter reads on the shared bus, switch the Modbus Bridge off using its hardware switch. External power may remain; its USB interface disappears when switched off. Adapter requests are blocked while a Modbus Bridge interface is detected. Only one Modbus master may drive the bus.

## Find devices and read values

Devices shows current or retained data. Read now requests a manual read. Last-good timestamps and stale labels identify old data after failures/disconnection; zero is retained when it is a genuine reading. The adapter detail view shows readings received through that connection. A profile match does not verify the physical sensor model. Register tables wrap long descriptions and offer unit cycling and native/display comparison where applicable. Unit changes do not rewrite sensor registers or Modbus Bridge tables.

Device details include every received register value. Live register tables highlight successful readings and show seconds since each register last succeeded; failed reads keep the prior value and increasing age. Registers without a successful reading have no age. When all entries for one slave fail, check power, wiring and serial settings as well as its register configuration.

The Devices map displays the verified configuration before a sensor scan finishes. Cards retain configured order and show slave-address badges. Reported point errors are attributed to their slave immediately; a global console timeout alone cannot identify which sensor failed. Configuration remains available after read failure, with **Check recovered console** to export the table without starting another scan. An interrupted write still requires recovery and review.

Use **Clear errors** beside **Refresh** to acknowledge displayed warnings and reset consecutive failure counts. This preserves diagnostic logs, last-good readings and safety blocks, and does not write to the instruments. New errors appear again when reported.

The Modbus Bridge card, device details and Configuration heading show its LoRa EUI after the console banner is read. **Copy EUI** copies exactly 16 uppercase hexadecimal characters, without spaces or separators. This identifier is separate from the USB serial number. It remains unavailable until the current bridge supplies a valid identifier.

Connections appear on the left and their slave devices on the right. Use **View device** on a card to open its measurements. The connection badge distinguishes ready, polling and communication failure. A ready bridge does not prove that every slave is responding.

## Interpret a custom register set

Selecting a device type for a custom register set opens the normal device summary with its image and reading cards. **Register table** opens the detailed readings for that slave. Units apply only to entries matching the selected model's register encoding.

Use **Custom profiles** to save a named interpretation for matching register sets on this computer. See [network configuration](MULTI-DEVICE-CONFIGURATION.md) for persistence and ambiguous matches.

## Configure the Modbus Bridge

1. Open Configuration and wait for a verified target. An orange banner identifies an unavailable target; hardware actions stay disabled.
2. Configuration starts with one device entry. Choose its model and slave address, then use **Add device** for more sensors (up to 32 devices and 32 register entries total). There is no separate single-device mode. You can also open a configuration file or saved backup: each slave becomes an editable entry, with unmatched rows preserved as a custom register set. Selection alone does not write.
3. Review changed/removed/added points and communication requirements. Edit Sensor slave address to match the physical sensor. Candidate profiles are not evidence of physical qualification. Use the separate RS-485 line settings editor for downstream baud, parity and framing; tab-separated configuration files (.tsv) do not contain those settings. See [Modbus Bridge line configuration](E5-LINE-CONFIGURATION.md).
4. Program Modbus Bridge queues behind an active automatic read. The application reads the identity and current table again, requires a successfully saved backup, writes with acknowledgements and verifies exported contents.
5. Inspect fresh readings after completion. On an interrupted or uncertain write, recover/check the console and review the saved backup before a restore. The application does not automatically restore an earlier configuration.

Metric HMD65 profile selection and its metric register-reference section are hidden. Use the HMD65 non-metric profile for new configurations. Existing tables, saved backups and received readings remain readable without conversion. IAQ reference and troubleshooting sections are hidden.


The selectable HMD65 non-metric profile uses one-based manual register numbers in its bridge table. Its eight entries include status at 513 and wet-bulb temperature at 141–142. It excludes 514–515, 518, 519 and enthalpy at 143–144. Reprogram an existing bridge with the updated profile to remove these entries from its reads. This convention applies to Modbus Bridge configuration files; direct Modbus requests and manufacturer reference addresses remain zero-based. Existing backups are preserved without conversion.

## Back up, load and restore

| Control | What it does |
|---|---|
| **Backup name** + **Back up Modbus Bridge** | Retrieve the current hardware point table and save a named copy. Reusing a name creates another version. |
| **Load backup** | Load a saved table for review. It does not write hardware. |
| **Program Modbus Bridge** | Apply the reviewed table, including a loaded backup. |
| **Save configuration file** / **Copy configuration table** | Save or copy the current selection, which may differ from the hardware table. |

Point-table backups exclude radio keys, serial framing, sensor calibration and firmware. Check the target and changed entries before restoring. Backup selection uses the USB connection identity; the LoRa EUI is a separate identifier.

## History and export

History contains up to 50,000 point samples from this session. Each recorded register has its own graph, labeled by slave address and source. Different register settings remain separate. Axes show native value units and elapsed seconds. Failures leave gaps. Export a comma-separated values file (.csv) before closing if the history is needed later.

For downstream measurement names and units, use the [DSP guide](../How%20do%20configure%20Modbus%20Bridge%20on%20DSP.md). Its tables assume one complete standard device profile per bridge.

## Appearance, units and polling

Settings: System/Light/Dark theme; System/United States/United Kingdom/Europe unit defaults; automatic polling. System units select the United States preset for that Windows region, the United Kingdom preset for that region, and the Europe preset for other or unavailable regions. The United States preset uses degrees Fahrenheit and pounds per square inch absolute where applicable. The United Kingdom preset uses degrees Celsius and bar; Europe uses degrees Celsius and kilopascals. Explicit register overrides last for the session. Polling off stops future automatic requests after the current operation finishes; it does not stop the Modbus Bridge from requesting readings on its serial bus.

Advanced communication settings control application waits, separately from the bridge serial-line settings. See [communication time limits](DIAGNOSTICS.md#adjust-application-time-limits).

## Help and saved files

References contains product information, register maps and Device manuals PDF buttons. Unavailable manuals are marked. Troubleshooting provides application guidance and device-specific material; instructions without established support show TBD. Diagnostics contains route selection, console operations, checks and log export. The compact activity list omits routine successful polling; persistent logs include background operation summaries.

Settings.json, Selected.tsv and Backups are stored beneath `%LOCALAPPDATA%/Polygon/Device Configurator/`. History itself is not persisted. Avoid sharing logs or configuration material that includes credentials.

Firmware updating is not available yet. See [firmware plan](FIRMWARE-UPDATES.md).

## Basis of the guidance

Modbus Bridge and Vaisala DPT146 instructions use recorded test results. Other device instructions are either explicitly attributed to manufacturer documentation or marked **TBD**. Documentation-based suggestions are not hardware verification. Historical notes and proposed test plans are not approved installation procedures.

See [terminology](TERMINOLOGY.md) for the distinction between an instrument register, a bridge entry, a profile and a backup.
