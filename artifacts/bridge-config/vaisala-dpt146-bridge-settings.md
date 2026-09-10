# Vaisala DPT146 settings for Polygon/Synetica ENL-MOD-32

Status: **validated on Polygon/Synetica ENL-MOD-32 firmware 3.6** on 2026-08-28.

## Bridge-wide Modbus settings

- Baud: 19200
- Data bits: 8
- Parity: None
- Stop bits: 2
- Retries: 1
- Timeout: 500 ms
- Inter-message delay: 150 ms

## Device settings

- Slave ID: 1
- Register type: Holding
- Register addressing in the ENL-MOD-32: zero-based PDU address
- 32-bit Vaisala values: high byte first within each word, low word first (`HL` in firmware 3.6)
- Multiplier: 1
- Read mode: Interval (`Int`)

## Corrected data points

| Item | Quantity | Address | Type | Word order | Unit |
|---:|---|---:|---|---|---|
| 1 | Temperature | 4 | F32 | HL | °C |
| 2 | Dew/frost point | 6 | F32 | HL | °C |
| 3 | Atmospheric-pressure dew/frost point | 10 | F32 | HL | °C |
| 4 | Moisture | 20 | F32 | HL | ppmv |
| 5 | Absolute pressure | 44 | F32 | HL | bara |
| 6 | Fault status | 512 | U16 | HH | 1 = no errors |
| 7 | Online status | 513 | U16 | HH | 1 = data available |
| 8 | Error code | 515 | U32 | HL | 0 = no errors |

## Addressing warning

The Vaisala manual shows both one-based logical register numbers and zero-based PDU addresses. ENL-MOD-32 firmware 3.6 transmits the address exactly as entered, so use the PDU addresses above. The original bridge table used the logical measurement addresses with `HH`, producing plausible-looking but incorrect values assembled from adjacent register words.

## E5 bridge validation result

The corrected table was imported into the bench E5 bridge and its detailed “Read All Data Points” test completed with **8 successful reads and 0 exceptions**.

| Quantity | Validated E5 bridge reading |
|---|---:|
| Temperature | 27.994858 °C |
| Dew/frost point | 9.879358 °C |
| Atmospheric-pressure dew/frost point | 10.090648 °C |
| Moisture | 12341.489258 ppmv |
| Absolute pressure | 0.999406 bara |
| Fault status | 1 (no errors) |
| Online status | 1 (data available) |
| Error code | 0 (no errors) |

The resulting table was exported again and matched `vaisala-dpt146-validated.tsv`.
