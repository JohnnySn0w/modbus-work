> Current addition: a separate non-metric float32 profile is available. See docs/devices/hmd65-representation-review.md for bank addresses and units. Physical word-order validation remains pending.

# HMD65 documentation-derived E5 bridge test

Status: **testable, not validated, not deployable**.

Import candidate: `hmd65-documentation-test.tsv` for ENL-MOD-32 firmware 3.6.

The table requests all eight documented metric floating-point measurements plus device, error, RH-quality, and temperature-quality status. Addresses are the zero-based PDU addresses explicitly published by Vaisala.

Before import, photograph the HMD65 DIP switches and confirm Modbus mode, slave address, bitrate, parity, and termination. The file uses slave ID 1 as a placeholder and assumes the all-off bitrate default of 19200. Change the ID to the physical DIP setting.

The documentation identifies IEEE 32-bit floats but does not explicitly settle the two-register word order. `HH` is the first candidate. If values are implausible, change only the eight float rows to `HL` and retest; do not alter their addresses. Status rows remain `HH`.

Expected healthy status values are zero. Initial testing is read-only.

