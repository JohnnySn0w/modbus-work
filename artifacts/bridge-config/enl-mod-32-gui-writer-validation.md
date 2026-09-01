# ENL-MOD-32 GUI writer validation

Validated on 2026-09-01 against the Polygon/Synetica ENL-MOD-32 firmware 3.6 bench bridge and Vaisala DPT146.

The new guarded writer used the Windows `.NET SerialPort` transport on COM5. The bridge was identified from its live banner as model `ENL-MOD-32`, firmware `3.6`. Before changing the point table, the transaction exported all eight existing rows to a live TSV backup. That export exactly matched the validated DPT146 table.

The transaction then used the firmware's supported Slave-ID-0 deletion method, imported the same eight DPT146 rows, required the bridge's acknowledgement for every submitted row, and exported the table again. Exported readback matched the selected TSV exactly.

The final Read All Data Points operation returned these live values:

| Item | Reading |
|---:|---:|
| 1 | 26.493454 |
| 2 | 9.6796 |
| 3 | 9.823129 |
| 4 | 12118.208008 |
| 5 | 1.003778 |
| 6 | 1 |
| 7 | 1 |
| 8 | 0 |

The console reported `Modbus read completed`, then its Modbus Configuration menu reported `8/0 (OK/Exceptions)`.

Firmware 3.6 delays the detailed Read All response until the STM32 CDC serial handle is reopened. The writer therefore keeps normal menu/import operations in one `.NET SerialPort` session, then performs one bounded close/reopen when the Read All completion text does not arrive on the original handle. This behavior was observed directly during validation and is now part of the firmware-specific bridge adapter.

The GUI apply workflow is compatibility-gated to exact model `ENL-MOD-32` and firmware `3.6`. It suspends background discovery, validates the selected eight-column TSV, asks where to save the live backup, requires operator confirmation, runs the transaction off the UI thread, and reports success only after acknowledgements, readback comparison, and Read All verification.
