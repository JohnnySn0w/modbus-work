# E5 bridge line settings and sensor addresses

Configuration contains two separate kinds of settings:

- **RS-485 line settings** control the E5 bridge's downstream serial bus.
- **Sensor slave address** controls the ID in the selected point-table TSV.

Neither changes the USB console framing or reconfigures the physical sensor.
All sensors sharing a bus must agree with the bridge's serial framing.

## Line settings

Expand RS-485 line settings in Configuration. Values come from the verified
E5 bridge menu, including during automatic reads. Read line settings provides
an explicit refresh when automatic polling is off. Edit values and use Apply
line settings; the request queues behind any current automatic read.

Observed ENL-MOD-32 firmware 3.6 options:

| Setting | Supported values |
| --- | --- |
| Baud rate | 2400, 4800, 9600, 14400, 19200, 38400, 56000, 57600 |
| Data bits | 7, 8 |
| Parity | None, Odd, Even |
| Stop bits | 1, 1.5, 2 |
| Retries | 0–10 |
| Response timeout | 10–20000 ms |
| Inter-message delay | 5–10000 ms |

Before any write, the application checks the current settings against the
reviewed values and saves the previous settings to a new JSON file under
`%LOCALAPPDATA%\Polygon\Device Configurator\Line settings`. Backup failure
prevents writes. Each changed field is selected from the firmware's actual
prompt and checked by reading the resulting menu. A failed or interrupted
verification pauses polling and requires a fresh check; partial changes are
never silently reported as success.

These JSON snapshots are separate from point-table TSV backups. TSV save/load,
programming and restore do not change line settings. There is currently no
one-click JSON line-settings restore; previous values can be reviewed in the
snapshot and reapplied through the editor.

## Sensor slave address

After choosing a point table, edit Sensor slave address in Review changes.
Addresses 1–247 are permitted; broadcast address 0 is excluded. For imported
tables containing multiple slave IDs, each existing ID has its own field.
Changing a field updates only rows using that ID. Register addresses, point
numbers, data types, word order and scaling are preserved.

The edited table is remembered locally. Save TSV, Copy TSV and Program E5 bridge
all use it. Choosing another bundled preset resets its fields to that preset's
defaults. Profile recognition permits consistent slave-ID remapping but rejects
mixing a profile's points across different sensor addresses.

## Register address notation

Register maps and live register pages offer **0-based (PDU)** and **1-based**
notation. This adds one to the displayed PDU address or range only. The Manual
column continues to reproduce the source documentation. Wire addresses, lookup,
TSV content and physical device configuration are unchanged.

## Verification, September 11, 2026

All seven menu prompts were inspected on the attached firmware 3.6 E5 bridge,
reselecting their current values. A first delay-change test stopped because it
did not yet handle the post-write Continue prompt. Readback confirmed 151 ms;
the handler was corrected and the original 150 ms setting was restored.

A subsequent complete 150 → 151 → 150 ms round trip passed, including snapshots
before both writes and an eight-value, zero-exception read afterward. Serial
framing and sensor slave addresses were not changed on the bench. Their menu
mappings and TSV transformations are covered by software tests, not physical
all-combinations validation. Raw traces remain in ignored local storage.
