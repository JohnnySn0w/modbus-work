# Vaisala DPT146 direct Modbus validation

Date: 2026-08-28

## Device

- Model: Vaisala DPT146
- Order code: B2MBD102A0X
- Serial number: U2720970
- Supply measured: approximately 20 VDC
- Connection: direct USB-COMi-TB on COM3, bridge master powered off

## Confirmed communications

- Protocol: Modbus RTU
- Slave ID: 1
- Baud: 19200
- Data bits: 8
- Parity: None
- Stop bits: 2
- Read function: 03, holding registers
- Float representation: IEEE-754 32-bit, least-significant 16-bit word first

The documented factory candidate `ID 240 / 19200 8E1` did not respond. No write requests were sent.

## Read results

| Quantity | Logical register | PDU start | Raw response | Decoded value |
|---|---:|---:|---|---:|
| Temperature | 5 | 4 | `01 03 04 83 28 41 DA E2 74` | 27.31 °C |
| Dew/frost point | 7 | 6 | `01 03 04 2D 7E 41 21 63 0F` | 10.07 °C |
| Atmospheric dew/frost point | 11 | 10 | `01 03 04 15 FE 41 24 AF 84` | 10.26 °C |
| Moisture | 21 | 20 | `01 03 04 04 95 46 43 98 BE` | 12481.15 ppmv |
| Absolute pressure | 45 | 44 | `01 03 04 20 C5 3F 80 F1 9E` | 1.00 bara |
| Fault status | 513 | 512 | `01 03 02 00 01 79 84` | 1 (no errors) |
| Online status | 514 | 513 | `01 03 02 00 00 B8 44` | 0 (data-not-available flag at test time) |
| Error code | 516-517 | 515 | `01 03 04 00 00 00 00 FA 33` | 0 (no errors) |

The online flag should be rechecked after the post-power-up stabilization period. Measurement responses and CRCs were valid.

