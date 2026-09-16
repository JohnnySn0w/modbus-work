> Historical evidence: observations and software state at the recorded test date. Earlier names, COM numbers, tool behavior and acceptance claims are preserved as evidence, not current instructions. See [current status](../../CURRENT-STATUS.md).

# ENL-MOD-32 configuration reset and restore validation

Validated on 2026-08-28 against Polygon ExactAire-Modbus Bridge / Synetica ENL-MOD-32 firmware 3.6.

## Menu assessment

All displayed main-menu branches were inspected: Quick Start, LoRa Radio Settings and Advanced, Modbus Configuration and Import/Export, Password and Security, Test Mode, and Reboot. Firmware 3.6 exposes a confirmed reboot operation but no labeled global factory-reset operation. The physical CONFIG button is documented as sending a LoRaWAN status message, not resetting the unit.

## Proven Modbus point reset method

The firmware 3.6 tab-delimited import screen states that setting a row's Slave ID to `0` deletes that configured item. Items 1 through 8 were submitted with Slave ID 0. Every row returned `deleted OK`, and the Import/Export menu subsequently reported `0/32` configured points.

This is the supported configuration-reset mechanism for the Modbus point table. It does not disturb LoRaWAN credentials or bridge-wide serial settings.

## Restore result

The eight rows from `vaisala-dpt146-validated.tsv` were imported again. Every row returned `imported OK`; the bridge reported `8/32` configured points.

A detailed Read All Data Points test then completed successfully:

- 8 successful reads;
- 0 exceptions;
- five plausible engineering measurements;
- fault status 1 (no faults);
- online status 1 (data available);
- error code 0.

## Persistence result

The bridge was rebooted through the console's confirmed reboot command. After USB re-enumeration on COM5, the Modbus menu still reported:

- 8/32 configured points;
- 8/0 successful reads/exceptions;
- 19200 baud, 8 data bits, no parity, 2 stop bits;
- 1 retry, 500 ms timeout, and 150 ms inter-message delay.

Result: point deletion, validated-table import, live readback, and reboot persistence are validated. A global factory reset is neither documented nor required for the clone/rollback workflow.
