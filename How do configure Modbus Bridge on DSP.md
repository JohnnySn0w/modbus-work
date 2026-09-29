


Go to the device's page
![[Pasted image 20260924153248.png]]

Go to Settings
![[Pasted image 20260924153218.png]]

Go to the Measurements section
![[Pasted image 20260924153159.png]]

There will be two sets of register labels. The first set is the "Int" set, and they're the primary use case. They read the same registers as the "Cum" set. The difference: there is a configuration in the bridge that allows for delayed data sends. The register is considered "cumulative" when in this mode. This is what the "Cum" registers are for.

Note that while it is possible to configure cumulative registers, defaults on the bridge are just for the first 32 registers that have "Int" at the end. It is safe to ignore the cumulative registers unless you specifically are using that mode.

For each register you can click the edit icon to configure how the reported data is interpreted.

![[Pasted image 20260924153525.png]]

![[Pasted image 20260924151603.png]]

The name field names the register.

The output type can be a numeric value, or it can be a lookup table. Lookup tables are most often useful for device errors enumeration, or for fields that do any form of encoding.
### Numeric Type
For a Numeric type, you can select the unit of measurement
![[Pasted image 20260924151720.png]]

Custom units allow free unit entry.
![[Pasted image 20260924153944.png]]


### Lookup Table
![[Pasted image 20260924153707.png]]
For a lookup table, the goal is to map the expected values to more easily read labels. In the case of an error field, it may convert codes to error explanations. Another example would be the Wattnode-WND-M1's register 1607, which reads different polarity in one register, and this is denoted by a bit flip. Much easier to read "CT 1 parity reversed, others normal" than it is to read "0b100".

## Tips
- For bridges that are running multiple slaves, it is useful to annotate the name of the register with the device type and slave ID like so:
	- "HMD65 - 2 - \[register label]"

HMD65

F12 ATI


---

## DSP entries for standard single-device configurations

Use the tables below when one instrument is configured on the Modbus Bridge using its complete, standard profile. Each table starts at bridge entry 1. These are alternative configurations, not sections to combine on one bridge.

**Match the bridge entry number, not the instrument register number.** For example, HMD65 temperature is bridge entry 2 and belongs to the second `Int` measurement in DSP; 131–132 are its instrument registers. A two-register floating-point value occupies one bridge entry and one DSP measurement.

The first column below identifies the corresponding numbered `Int` measurement. The exact field name in DSP may include a prefix. The instrument register column is for cross-checking only; do not enter it as the DSP measurement number. If a decoder displays raw zero-based point indices, index 0 corresponds to bridge entry 1.

1. Confirm that the installed bridge table matches the full profile below, including entry order. Removing or reordering entries changes the DSP mapping.
2. Edit each numbered `Int` measurement. Enter the suggested name, output type and unit.
3. Use a custom unit if the required unit is unavailable. Leave status and raw-code fields without an engineering unit unless a custom unit is specified below.
4. Do not add a second scale factor to these standard profile values: their bridge multiplier is 1. Selecting a unit label must not be assumed to convert the incoming value. For example, DPT146 temperature arrives in °C; labeling it °F alone would be incorrect.
5. Leave unused entries and the `Cum` fields unconfigured for these standard `Int` profiles. A cumulative energy value such as WattNode energy total still uses its assigned `Int` field here.

Names can omit the device prefix when the DSP device name already identifies the instrument. The tables retain it to make exported measurements clear.


### Vaisala DPT146

**8 bridge entries; use `Int` entries 1–8.**

| Bridge entry (`Int`) | Suggested DSP name | Output type | Unit / custom unit | Instrument register(s) |
|---:|---|---|---|---|
| 1 | DPT146 — Temperature | Numeric | °C | 5–6 |
| 2 | DPT146 — Dew/frost point | Numeric | °C | 7–8 |
| 3 | DPT146 — Atmospheric dew point | Numeric | °C | 11–12 |
| 4 | DPT146 — Moisture | Numeric | ppmv | 21–22 |
| 5 | DPT146 — Absolute pressure | Numeric | bar absolute | 45–46 |
| 6 | DPT146 — Fault status | Lookup table | None | 513 |
| 7 | DPT146 — Online status | Lookup table | None | 514 |
| 8 | DPT146 — Error code | Numeric (raw code) | None | 516–517 |

#### DPT146 lookup entries

| Bridge entry | Incoming value | DSP label |
|---:|---:|---|
| 6 — Fault status | 0 | One or more errors active |
| 6 — Fault status | 1 | No errors |
| 7 — Online status | 0 | Online data unavailable |
| 7 — Online status | 1 | Online data available |

For entry 8, retain the numeric error code. A value of 0 means no errors; a nonzero value requires interpretation against the DPT146 documentation. Do not map every nonzero value to a specific fault without its documented definition.


### Vaisala HMD65 — non-metric

**8 bridge entries; use `Int` entries 1–8.**

| Bridge entry (`Int`) | Suggested DSP name | Output type | Unit / custom unit | Instrument register(s) |
|---:|---|---|---|---|
| 1 | HMD65 — Relative humidity | Numeric | %RH | 129–130 |
| 2 | HMD65 — Temperature | Numeric | °F | 131–132 |
| 3 | HMD65 — Dew point | Numeric | °F | 133–134 |
| 4 | HMD65 — Dew/frost point | Numeric | °F | 135–136 |
| 5 | HMD65 — Absolute humidity | Numeric | gr/ft³ | 137–138 |
| 6 | HMD65 — Mixing ratio | Numeric | gr/lb | 139–140 |
| 7 | HMD65 — Wet-bulb temperature | Numeric | °F | 141–142 |
| 8 | HMD65 — Device status | Numeric (bit mask) | None | 513 |

`gr` means **grains**, not grams. Do not substitute g/m³ or g/kg for gr/ft³ or gr/lb without an actual conversion.

Entry 8 is a bit mask. Keep it numeric unless DSP is configured to decode combinations of flags. These values describe individual flags, not an exhaustive exact-value lookup:

| Flag value, decimal | Meaning |
|---:|---|
| 0 | Status OK — no flags set |
| 1 | Critical error; maintenance needed |
| 2 | Error; device may recover |
| 4 | Warning |
| 8 | Notification |
| 16 | Calibration mode active |

For example, 6 combines 2 and 4: error plus warning. An exact-value lookup containing only 1, 2, 4, 8 and 16 would not explain that value.

This profile does **not** include enthalpy at 143–144, error code at 514–515, humidity status at 518, or temperature status at 519. Register 142 is the second word of wet-bulb temperature at 141–142; it is included in bridge entry 7.

Measurement units and register definitions: [Vaisala HMD65 measurement data registers](https://docs.vaisala.com/r/M212264EN-B/en-US/GUID-BD253180-F696-448F-906C-0528DA9AED31/GUID-3A122B29-E8C5-4EA7-9A15-BDDD2102A3D8).


### ATI F12 — peracetic acid sensor

**13 bridge entries; use `Int` entries 1–13.**

The concentration fields below use **ppm only if the installed peracetic acid sensor is configured to report ppm**. Confirm the unit and full-scale range on the transmitter. If the transmitter uses another unit, use that unit for entries 5, 8 and 13. Entry 8 is the primary displayed gas concentration; keep the blanked and unblanked values clearly distinguished.

| Bridge entry (`Int`) | Suggested DSP name | Output type | Unit / custom unit | Instrument register(s) |
|---:|---|---|---|---|
| 1 | F12 — Expanded fault flags | Numeric (bit mask) | None | 33 |
| 2 | F12 — Expanded status flags | Numeric (bit mask) | None | 34 |
| 3 | F12 — Fault flags | Numeric (bit mask) | None | 35 |
| 4 | F12 — Status flags | Numeric (bit mask) | None | 36 |
| 5 | F12 — Unblanked concentration | Numeric | ppm — verify sensor unit | 37–38 |
| 6 | F12 — Unblanked concentration percent full scale | Numeric | % | 39–40 |
| 7 | F12 — Temperature | Numeric | °C | 41–42 |
| 8 | F12 — Blanked concentration | Numeric | ppm — verify sensor unit | 43–44 |
| 9 | F12 — Blanked concentration percent full scale | Numeric | % | 45–46 |
| 10 | F12 — Loop output | Numeric | mA | 47–48 |
| 11 | F12 — DAC output | Numeric | count | 51 |
| 12 | F12 — Scaled concentration — raw count | Numeric | count | 52 |
| 13 | F12 — Concentration full-scale range | Numeric | ppm — verify sensor unit | 73–74 |

The instrument register numbers above omit the holding-register `40000` prefix: 33 is also written as 40033 in the interface manual. They are not DSP entry numbers.

**Entries 1–4 contain independent flags that can be combined.** Numeric output preserves every reported value. Use an exact-value lookup only when the required combinations have been defined; do not treat individual bit values as a complete list of possible states.

| Bridge entry | Meaning of zero | Interpretation of nonzero values |
|---:|---|---|
| 1 — Expanded fault flags | No expanded fault flags set | Decode the expanded fault mask |
| 2 — Expanded status flags | No expanded status flags set | Decode status flags; these are not all faults |
| 3 — Fault flags | No fault flags set | Decode the fault mask |
| 4 — Status flags | No status flags set | Decode status flags, including alarm and inhibit states |

For entry 12, preserve the raw count. Documented reference values are 0 = fault, 300 = inhibit, 800 = 0% of full scale, and 4000 = 100% of full scale. Values between 800 and 4000 represent intermediate concentrations, so those four labels alone are not a complete lookup table. Use entry 8 for the main gas-concentration measurement.

The complete profile has 13 entries. A shortened 10-entry table will not necessarily have the same numbering. Match the actual exported bridge table before applying these labels.

Flag definitions and concentration behavior are documented in the D12/F12 Modbus interface manual, available under [Badger Meter F12/D product documentation](https://www.badgermeter.com/products/gas-monitoring/detectors/f12d-toxic-gas-detector/).


### WattNode WND-M1-MB

**12 bridge entries; use `Int` entries 1–12.**

| Bridge entry (`Int`) | Suggested DSP name | Output type | Unit / custom unit | Instrument register(s) |
|---:|---|---|---|---|
| 1 | WattNode — Energy total | Numeric | kWh | 1001–1002 |
| 2 | WattNode — Active power total | Numeric | W | 1009–1010 |
| 3 | WattNode — Active power 1 | Numeric | W | 1011–1012 |
| 4 | WattNode — Active power 2 | Numeric | W | 1013–1014 |
| 5 | WattNode — Active power 3 | Numeric | W | 1015–1016 |
| 6 | WattNode — Voltage AN | Numeric | V | 1019–1020 |
| 7 | WattNode — Voltage BN | Numeric | V | 1021–1022 |
| 8 | WattNode — Voltage CN | Numeric | V | 1023–1024 |
| 9 | WattNode — Frequency | Numeric | Hz | 1033–1034 |
| 10 | WattNode — Current 1 | Numeric | A | 1163–1164 |
| 11 | WattNode — Current 2 | Numeric | A | 1165–1166 |
| 12 | WattNode — Current 3 | Numeric | A | 1167–1168 |

Voltage AN, BN and CN are phase-to-neutral values. Use the labels that match those measurements; do not label them as phase-to-phase voltage.

The standard profile contains numeric energy, power, voltage, frequency and current values. It does **not** include current-transformer direction register 1607, so no direction lookup is needed for this configuration.

**Correction to the earlier lookup example:** direction mask 1 (`0b001`) reverses current transformer 1, mask 2 (`0b010`) reverses current transformer 2, and mask 4 (`0b100`) reverses current transformer 3. This indicates configured polarity reversal, not automatic detection of a physically reversed transformer. Multiple flags add together. See the [WattNode Module reference manual](https://ctlsys.com/wp-content/uploads/2017/08/WND-Module-Modbus-Ref-Manual.pdf).


### Final check

Compare each configured DSP measurement with a fresh Modbus Bridge reading for the same numbered entry. Confirm the label, unit and status interpretation. A missing or failed read is not a zero measurement. If the bridge point table is later changed, review these DSP labels again.

These mappings use the complete standard profiles in Polygon Device Configurator as of 29 September 2026. Source files: `vaisala-dpt146-validated.tsv`, `hmd65-nonmetric-documentation-test.tsv`, `ati-badger-f12-d12-documentation-test.tsv`, and `wattnode-wnd-m1-mb-documentation-test.tsv`.
