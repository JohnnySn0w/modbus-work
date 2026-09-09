# Remaining device register assessment

These tables are documentation-derived starting points. They have not been bench-tested with the listed hardware.

| Device | Table status | Integration note |
|---|---|---|
| ATI / Badger Meter F12 or D12 | Core measurement and identity registers captured | Confirm gas-specific units and installed firmware before creating a write profile. |
| Micronics U1000MKII-HM | Complete documented 32-register read map captured | Modbus RTU defaults are address 1, 38400 baud, no parity, and 2 stop bits. Minimum poll interval is 1 second. |
| Micronics U3000 / UF3300 | Operational and diagnostic subset captured | The supplied product reference resolves to UF3300 documentation. Confirm the exact nameplate model and manual revision before bench testing. |
| Precision Digital PD2-6000 | Core readout, totalizer, status, and identity blocks captured | The full protocol manual contains a much larger configuration map. Writes should be added only for the installed option set. |
| Lighthouse Solair 1100LD | Core control and particle-record registers captured | The documented serial protocol is Modbus ASCII at 19200 8N1, not Modbus RTU. Treat bridge compatibility as unverified. |
| Seeed SenseCAP ONE S200 | Complete S200 wind-measurement map and common communications settings captured | Documentation lists S200 address 44 and 9600 8N1. Read measurements with function 04. |
| RKI VOC Pro | Documented operating, calibration, fault, and relay registers captured | Confirm sensor type and gas-specific units from the installed instrument. |
| Advantech ADAM-4053 | Digital inputs and identity registers captured | Current documentation lists Modbus RTU support, while older manual revisions conflict. Track hardware and firmware revision during testing. |

## Source documents

- ATI / Badger Meter: D12/F12 Modbus Interface User Manual, Rev G.
- Micronics U1000MKII: U1000MKII User Manual, Issue 3.1.
- Micronics UF3300: UF3300 User Manual and Modbus Supplement, Issue 1.1.
- Precision Digital: Modbus Register Tables for ProVu and ProtEX-MAX meters.
- Lighthouse Worldwide Solutions: SOLAIR 1100LD Operating Manual, Modbus map version 1.48.
- Seeed Studio: SenseCAP ONE Compact Weather Sensor User Guide.
- RKI Instruments: VOC Pro Operator's Manual, Appendix C.
- Advantech: ADAM-4000 Series User Manual and current ADAM-4053 product data.

## Next validation pass

For each physical unit, record the exact model suffix, firmware version, slave address, serial settings, successful identity fingerprint, verified read registers, and any writable configuration values. A device moves from documentation-derived to tested only after a read-only scan succeeds and decoded values agree with its local display or a reference instrument.
