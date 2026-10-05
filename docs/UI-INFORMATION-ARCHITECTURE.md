# UI information architecture

| Section | Purpose |
|---|---|
| Devices | Actual/previous equipment, transport, readings, timestamps and per-device details |
| Configuration | Modbus Bridge target, selected point table, review, backup/program/save |
| History | One graph per register and slave, native units, gaps and CSV export |
| References | Supported model catalog, register definitions and offline manuals |
| Troubleshooting | Application and device setup/troubleshooting guidance |
| Settings | Theme, display units, automatic polling and communication time limits |
| Diagnostics | USB interfaces, explicit recovery, data checks, communication timeline and diagnostic export |

Navigation remains outside page scrolling. Scrollbars fill the viewport width with independent content padding. Register columns keep shared widths and rows grow to the tallest wrapped cell. The bottom status bar stays fixed; automated poll progress does not move content or disable configuration selection. Explicit work queues behind polling.

A USB interface is a route, a Modbus Bridge is physical equipment, and a sensor is the measurement source. The adapter detail view displays its sensor readings. Profiles describe desired configuration and do not establish physical identity. Unidentified Synetica USB and identified Modbus Bridge states do not create duplicate device entries. Reference photos are not live evidence.

See [operator guide](OPERATOR-GUIDE.md) for controls and [current status](../CURRENT-STATUS.md) for availability.
