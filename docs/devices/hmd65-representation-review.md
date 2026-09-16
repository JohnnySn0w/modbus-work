> Research notes, not operating instructions. Recommendations without cited technical support are unverified. Hardware-specific setup and troubleshooting: **TBD** unless a step is explicitly supported by a cited manufacturer source.

# HMD65 representations and ATI F12/PAA support

Updated 2026-09-10. Documentation candidates; remote hardware verification remains pending. No hardware programming was performed for this change.

## HMD65

Vaisala M212264EN-B confirms both banks. Manual addresses are one greater than zero-based Modbus PDU addresses.

| Measurement | Metric float32 | Non-metric float32 | Non-metric unit |
|---|---|---|---|
| Relative humidity | 1-2 | 129-130 | %RH |
| Temperature | 3-4 | 131-132 | degrees F |
| Dew point | 5-6 | 133-134 | degrees F |
| Dew/frost point | 7-8 | 135-136 | degrees F |
| Absolute humidity | 9-10 | 137-138 | grains/ft3 |
| Mixing ratio | 11-12 | 139-140 | grains/lb |
| Wet bulb | 13-14 | 141-142 | degrees F |
| Enthalpy | 15-16 | 143-144 | Btu/lb |

Device status is 513, error code is **514-515** (two words), RH status is 518, and temperature status is 519. Status/error registers represent flags; signed transport types do not change that meaning.

Prefer native IEEE-754 float32 where available. It preserves the native representation and avoids the quantization and saturation of scaled integer registers. It does not improve sensor accuracy. Each float uses two Modbus words. Integer banks use a scale factor of 100 and can saturate at 32767.

The application offers separate metric and non-metric configuration profiles. The display-unit controls and optional native-value comparison change presentation only; they do not rewrite registers or saved native data. HH word order remains unverified for HMD65 floating-point measurements. The error-code pair at 514-515 is excluded from Modbus Bridge tables. Both Modbus Bridge profiles use one-based manual register numbers and contain 11 entries. This configuration convention does not change the manufacturer reference addresses or direct Modbus requests.

Source: https://docs.vaisala.com/r/M212264EN-B/en-US/GUID-BD253180-F696-448F-906C-0528DA9AED31

## ATI F12 with PAA module

The initial profile uses the reviewed D12/F12 Modbus map: 13 Modbus Bridge points, low-word-first float32 (HL), concentration, temperature, loop output and status. The original 40001-style register notation is normalized to zero-based PDU addresses; primary concentration 40043-40044 is PDU 42-43.

The transmitter register map is separate from the PAA sensor variant so additional gases can be added later. Concentration units and range must be verified against the installed sensor; the application does not assume ppm. Defaults documented in the source are address 1, 9600 baud, 8N1; a TSV does not configure serial framing.

This adds a Modbus Bridge configuration candidate and reference/decoding support. Direct USB adapter identification and polling of ATI F12 is not implemented. Remote hardware verification is pending. The F12 product photograph is embedded in the reference and device views (2026-09-10). The photo does not establish the installed sensor configuration.

Source: https://www.analyticaltechnology.com/wp-content/uploads/2022/02/D12-F12-Modbus-Manual.pdf
Reviewed local table: artifacts/register-tables/ati-badger-f12-d12-modbus-register-table.csv
