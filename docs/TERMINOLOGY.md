# Application terminology

| Term | Meaning |
|---|---|
| Connection / interface | A USB or serial route. Its presence does not prove an instrument is responding. |
| Modbus Bridge | The bridge device. Use its manufacturer and model when identifying hardware or a source document. |
| Slave address | An instrument's address on the shared bus. Editing a point table changes requests, not the instrument's stored address. |
| Instrument register | A manufacturer-defined location. A measurement can occupy multiple registers. |
| Bridge entry / point | One configured bridge read. A two-register float is one entry. DSP numbering follows these entries. |
| Point table | The complete list of requests, addresses, encodings, multipliers and read modes. |
| Standard profile | A supplied model-specific table. It does not establish physical sensor identity. |
| Custom profile | A saved interpretation of an existing register set. It does not program new register definitions. |
| Backup | A saved hardware point table, not a complete copy of every device setting. |
| Load | Prepare a saved table for review without writing hardware. |
| Program / restore | Write and verify a reviewed table. A restore uses a previously saved table. |
| Native value | The received value before display-unit conversion. |
| Last-good value | The latest successful reading, retained with its timestamp after later failures. |
| Read age | Seconds since that register last succeeded, not since the scan began. |
| Configured | Describes a selected model or request; does not mean connected or verified. |
| Verified table | A table confirmed by export; does not establish sensor identity or accuracy. |
| Confirmed operation | Recorded operation on specified hardware; does not qualify every register or revision. |
| Activity log | The compact on-screen event list for the current session. |
| Diagnostic export | Retained process logs and diagnostic context for support. |

Use exact control labels in instructions. Preserve manufacturer names and protocol terms when they identify a specific field; explain them nearby when needed. Use **TBD** for unavailable procedures rather than unverified instructions.
