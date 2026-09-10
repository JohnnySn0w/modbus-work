# Operator guide

## Start and connect

Extract the current Windows ZIP and run Polygon Device Configurator.exe. Close other instances before starting a replacement. The app automatically discovers routes and polls supported devices; COM numbers may change. Verify the physical model and wiring independently of any selected E5 profile.

For direct USB adapter reads on the shared bus, switch the E5 bridge off using its hardware switch. External power may remain; its USB interface disappears when switched off. Adapter requests are blocked while an E5 interface is detected. Only one Modbus master may drive the bus.

## Readings

Devices shows current or retained data. Read now requests a manual read. Last-good timestamps and stale labels identify old data after failures/disconnection; zero is retained when it is a genuine reading. The adapter detail view shows the sensor identified through that route. Register tables wrap long descriptions and offer unit cycling and native/display comparison where applicable. Unit changes do not rewrite sensor registers or E5 tables.

## Configure an E5 bridge

1. Open Configuration and wait for a verified target. An orange banner identifies an unavailable target; hardware actions stay disabled.
2. Select a sensor profile, Open TSV file, or a saved backup. Selection alone does not write.
3. Review changed/removed/added points and communication requirements. Candidate profiles are not evidence of physical qualification. A TSV does not set downstream baud/parity/stop bits.
4. Program E5 bridge queues behind an active automatic read. The app refreshes identity/table, requires a durable backup, writes with acknowledgements and verifies exported contents.
5. Inspect fresh readings after completion. On an interrupted or uncertain write, recover/check the console and review the saved backup before a restore. There is no blind automatic rollback.

Back up now retrieves the point table. Backups lists per-USB-identity versions. Save TSV file and Copy TSV use the current selection. Point-table backups exclude radio keys, serial framing, sensor calibration and firmware.

## History, settings and help

History contains up to 50,000 point samples from this session. Choose the measurement/route; axes show native value units and elapsed seconds. Failures leave gaps. Export CSV before closing if the history is needed later.

Settings: System/Light/Dark theme; System/US/UK/EU unit defaults; automatic polling. System units resolve US to US, GB to UK, other/unavailable regions to EU. US uses Fahrenheit and psia where applicable; UK uses Celsius/bar; EU uses Celsius/kPa. Explicit register overrides last for the session. Polling off stops future automatic requests after the current operation finishes; it does not disable E5 downstream traffic.

References contains product information, register maps and Device manuals PDF buttons. Unavailable manuals are marked. Troubleshooting provides application guidance and device-specific material; unqualified devices may show TBD. Diagnostics contains route and activity details; automatic polling is omitted from the activity log.

Settings.json, Selected.tsv and Backups are stored beneath `%LOCALAPPDATA%/Polygon/Device Configurator/`. History itself is not persisted. Avoid sharing logs or configuration material that includes credentials.

Firmware updating is not available yet. See [firmware plan](FIRMWARE-UPDATES.md).
