# WattNode WND-M1-MB documentation-derived bridge test

Status: **testable, not validated, not deployable**.

Import candidate: `wattnode-wnd-m1-mb-documentation-test.tsv` for ENL-MOD-32 firmware 3.6.

The 12-point table uses native floating-point registers: total energy; total and per-element active power; phase-to-neutral voltages; frequency; and three CT currents. The WND-M1-MB manual specifies low 16-bit word first, represented by `HL` in the validated ENL-MOD-32 firmware 3.6 convention.

The file uses slave ID 1 as a placeholder. The module has no address/baud DIP switches and may be factory ordered with different communication options. Read the front-label option text and establish the actual address, baud, parity, and stop bits before import. For an unlabeled/default unit, make read-only first contact at 19200 8N1 using address 1, then address 127 as the bounded legacy-firmware fallback. Do not sweep the full address range on an operating bus.

This measurement table does not require `CurrentIntScale` or `PowerIntScale`. CT ratings, voltage/CT mapping, and connection type still affect the physical measurements and must be read and verified.

Initial testing is read-only. Do not zero energy or demand and do not modify CT, gain, phase, mapping, or communication registers during first contact.
