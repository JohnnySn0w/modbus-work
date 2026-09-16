# Documentation-derived Modbus Bridge test profiles

These TSV files are native ENL-MOD-32 point tables assembled from manufacturer register maps. They are bench-test starting points, not field-validated configurations. Import only the profile for the connected device, then run the Modbus Bridge detailed read test and compare the returned values with the local display or a known reference.

| Profile | Default serial settings used | Address basis | Word setting | Bench notes |
| --- | --- | --- | --- | --- |
| ATI / Badger Meter F12/D12 | Slave 1; 9600; 8N1 | Manual 40001 notation converted to zero-based PDU offsets | HL for 32-bit rows | Manufacturer examples show high byte first within each register and low register first. Confirm the installed transmitter still uses its factory serial settings. |
| Micronics U1000MKII-HM | Slave 1; configure Modbus Bridge to match device; 8N2 is supported | Manual offsets used directly | HH | Manufacturer specifies AB CD, most-significant byte first. The unit responds only while an operating readout screen is displayed. Minimum poll interval is 1 second. |
| Micronics U3000 / UF3300 | Slave 1; 19200; 8E1 | Manufacturer register index used directly | HH | Factory frame is 19200 8E1. Confirm the device address in its Modbus setup menu. |
| Precision Digital PD2-6000 | Slave 1; confirm baud and parity on the meter | Manual 40001 notation converted to zero-based PDU offsets | HH | Manufacturer specifies highest byte first. Read-only measurement and status points are included. |
| RKI VOC Pro | Slave 1; 9600; 8N1 | Published decimal addresses used as written | LL for float rows | Manufacturer specifies least-significant bytes first for floating-point values. Confirm LL with a known gas or voltage value during bench validation. |
| Seeed SenseCAP S200 | Slave 44; 9600; 8N1 | Published hexadecimal PDU addresses converted to decimal | HH | Uses input registers for wind measurements. Values are scaled by 0.001. |

The bridge-wide serial settings are not contained in the TSV export. Set baud, data bits, parity, and stop bits separately before testing. If a profile fails with valid wiring and slave ID, verify the word setting against a known value before changing register addresses.
