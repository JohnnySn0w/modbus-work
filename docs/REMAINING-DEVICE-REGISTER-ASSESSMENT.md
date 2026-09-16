> Research notes, not operating instructions. Recommendations without cited technical support are unverified. Hardware-specific setup and troubleshooting: **TBD** unless a step is explicitly supported by a cited manufacturer source.

> Review 2026-09-10: documentation-derived register/bench planning material. Physical HMD65, WattNode and ATI acceptance remains pending. ATI F12/PAA now has a native Modbus Bridge profile; direct ATI adapter support is still pending. Current support: [status](../CURRENT-STATUS.md).

# Remaining device register assessment

These tables are documentation-derived starting points. They have not been bench-tested with the listed hardware.

| Device | Table status | Integration note |
|---|---|---|
| ATI / Badger Meter F12 or D12 | Bridge-readable numeric measurement and status registers captured | Text gas-name and unit fields are omitted because the bridge does not support ASCII values. Confirm gas-specific units from the instrument label and setup record. |
| Micronics U1000MKII-HM | Bridge-readable numeric map captured | The three-word serial identifier is omitted because the bridge accepts only one, two, or four words. Modbus RTU defaults are address 1, 38400 baud, no parity, and 2 stop bits. Minimum poll interval is 1 second. |
| Micronics U3000 / UF3300 | Operational and diagnostic subset captured | The supplied product reference resolves to UF3300 documentation. Confirm the exact nameplate model and manual revision before bench testing. |
| Precision Digital PD2-6000 | Bridge-readable numeric readout, totalizer, and status blocks captured | The nonnumeric identification block is omitted. The full protocol manual contains a much larger configuration map. |
| Seeed SenseCAP ONE S200 | Complete S200 wind-measurement map and common communications settings captured | Documentation lists S200 address 44 and 9600 8N1. Read measurements with function 04. |
| RKI VOC Pro | Bridge-readable operating, calibration, fault, and relay registers captured | Relay-reset command registers are omitted. Confirm sensor type and gas-specific units from the installed instrument. |

## Source documents

- ATI / Badger Meter: D12/F12 Modbus Interface User Manual, Rev G.
- Micronics U1000MKII: U1000MKII User Manual, Issue 3.1.
- Micronics UF3300: UF3300 User Manual and Modbus Supplement, Issue 1.1.
- Precision Digital: Modbus Register Tables for ProVu and ProtEX-MAX meters.
- Lighthouse Worldwide Solutions: SOLAIR 1100LD Operating Manual, Modbus map version 1.48.
- Seeed Studio: SenseCAP ONE Compact Weather Sensor User Guide.
- RKI Instruments: VOC Pro Operator's Manual, Appendix C.

## Excluded device

The Advantech ADAM-4053 is intentionally excluded. Its useful field data is exposed as coil/discrete I/O, while the Synetica Modbus Bridge supports only function 3 and function 4 reads from holding and input registers. The bridge does not support coils or discrete inputs, so a readable identity block would not provide the required channel data.

The Lighthouse Solair 1100LD is intentionally excluded from bridge configurations and bridge-facing register tables. Its documented serial interface uses Modbus ASCII. The Modbus Bridge requires Modbus RTU.

## Bridge table inclusion rules

A bridge-facing table includes only values that meet all of these conditions:

- Modbus RTU over two-wire RS485.
- Function 3 holding-register or function 4 input-register read.
- Unsigned integer, signed integer, or floating-point numeric value.
- One, two, or four consecutive 16-bit words.
- Read-only acquisition; no coil, discrete-input, text, command, or write operation.

## Next validation pass

For each physical unit, record the exact model suffix, firmware version, slave address, serial settings, successful identity fingerprint, verified read registers, and any writable configuration values. A device moves from documentation-derived to tested only after a read-only scan succeeds and decoded values agree with its local display or a reference instrument.
