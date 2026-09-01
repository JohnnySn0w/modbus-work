# ENL-MOD-32 LoRaWAN payload contract

Status: validated for Polygon/Synetica ENL-MOD-32 firmware 3.6 with the Vaisala DPT146 reference configuration.

## Captured Loriot uplink

- Timestamp: 2026-08-28 5:46:10 PM (Loriot display time)
- DevEUI: `0004A30B0005CC7C`
- JoinEUI: `53796E0000000000`
- Frame counter: 18
- FPort: 1
- Frequency: 903.300 MHz
- Spreading factor: 9
- Bandwidth: 125 kHz
- RSSI: -38 dBm
- SNR: 13 dB
- Gateway: `000800FFFF4BFE7D`
- Evidence: `docs/evidence/loriot-dpt146-uplink-fcnt18.png`

Raw application payload:

```text
10 00 41 BF AE 28
10 01 41 05 F9 59
10 02 41 07 0D 54
10 03 46 2C 53 C6
10 04 3F 81 0A 69
10 05 3F 80 00 00
10 06 3F 80 00 00
10 07 00 00 00 00
```

## Encoding

For this packet, the payload is a sequence of six-byte records:

| Offset | Length | Meaning |
|---:|---:|---|
| 0 | 1 | Record/type marker `0x10` |
| 1 | 1 | Zero-based configured data-point index |
| 2 | 4 | IEEE-754 float32, big-endian byte order |

Decoder pseudocode:

```text
for each 6-byte record:
    require record[0] == 0x10
    point_index = record[1]
    value = float32_big_endian(record[2:6])
```

The radio payload carries every configured point as float32, including integer/boolean Modbus source types.

## Decoded DPT146 points

| Index | Meaning | Decoded value | Unit/semantics |
|---:|---|---:|---|
| 0 | Temperature | 23.960037 | °C |
| 1 | Dew/frost point | 8.373376 | °C |
| 2 | Atmospheric-pressure dew/frost point | 8.440754 | °C |
| 3 | Moisture | 11028.943359 | ppmv |
| 4 | Absolute pressure | 1.008130 | bara |
| 5 | Fault status | 1 | 1 = no errors |
| 6 | Online status | 1 | 1 = data available |
| 7 | Error code | 0 | 0 = no errors |

## Decoder requirements

- Map indices using the configuration manifest; never assume an index meaning without the matching validated configuration/version.
- Reject truncated payloads and records whose marker is not `0x10`.
- Preserve the raw payload, frame counter, receive timestamp, FPort, and decoder version with decoded data.
- Treat status points as numeric float values on the wire, then coerce them according to the device profile.
- Firmware or configuration changes require payload revalidation.
