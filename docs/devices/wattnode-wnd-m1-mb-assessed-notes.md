# WattNode WND-M1-MB assessed setup notes

Status: documentation assessment; bench validation not yet performed.

Machine-readable future bench profile: `artifacts/device-profiles/to-test/wattnode-wnd-m1-mb.yaml`. It is deliberately marked non-deployable and contains promotion gates for later hardware testing.

Target family: Continental Control Systems WattNode WND-M1-MB. Confirm the exact model, serial number, hardware revision, and firmware before applying a profile or writing configuration.

## Scope boundary

For the initial device proofs of concept, acceptance ends when the bridge produces a correctly decoded raw payload in Loriot. Routing from Loriot into DSP is normally provided by the existing platform and is outside this project's present scope.

## Correction to the legacy scaling note

`CurrentIntScale` (register 1622 in the cited WattNode map) is not expressed in milliamps. It is the full-scale integer count used by the integer current registers.

```text
current_amps = integer_current_value * CtAmpsX / CurrentIntScale
```

The manual's example uses a 200 A CT and `CurrentIntScale = 20000`, which yields `0.01 A/count`. With a 250 A CT and the same scale, the multiplier is instead:

```text
250 / 20000 = 0.0125 A/count
```

To obtain exactly `0.01 A/count` with a 250 A CT, `CurrentIntScale` would have to be 25000. Do not assume this was done on an existing meter: read back the register and record it.

Preferred POC approach: use WattNode floating-point measurement registers where the bridge supports them. Float current values are already in amperes and float power values are already in watts, avoiding integer scale factors and portal-side multipliers.

## Configuration classification

| Setting | What it means | Normal technician action |
|---|---|---|
| `CtAmps` / `CtAmps1..3` | Rated primary current of the installed CTs | Required installation input; configure from the CT label and read back |
| `CurrentIntScale` (1622) | Full-scale count for integer current registers; default 20000 | Leave at default unless a legacy integer pipeline has a documented scaling requirement |
| `PowerIntScale` | Scaling for integer power registers | Prefer auto/default or avoid by reading float power registers |
| `GainAdjust1..3` (1612–1614) | Per-phase gain correction; 10000 means 1.0000/no correction | Calibration-only; change only from traceable reference-meter or CT correction data |
| `PhaseAdjust1..3` (1615–1617) | Per-phase CT phase-angle correction in 0.001 degree increments | Calibration-only; change only from CT characterization data |
| `NominalCtVolts1..3` (1629–1634) | CT secondary voltage at rated primary current | Leave at 0.33333 Vac for standard CCS 0.333 V CTs; change only for a different CT output or burden arrangement |

`GainAdjust` relationship:

```text
gain_multiplier = GainAdjustX / 10000
```

Examples: 10200 is +2%; 9800 is -2%.

For `PhaseAdjust`, a CT characterized as having a 0.600-degree phase lead would use -600. This is not a field guess or a routine commissioning tweak.

The CT amp rating and nominal CT output voltage are different facts. A standard 250 A / 0.333 V CT is configured as 250 A while the nominal CT voltage remains 0.33333 V.

## Proposed 250 A standard-CT baseline

This is a candidate, not yet a validated configuration:

- exact meter model and firmware: capture first;
- CT rating per phase: 250 A, but verify each physical CT label;
- nominal CT voltage: 0.33333 Vac, only if the installed CTs are standard 0.333 V models;
- gain adjustment: 10000 on all phases unless documented calibration corrections exist;
- phase adjustment: preserve and read back the physical meter values; WND-M1-MB reference manual 1.10 documents a default of -1000 on all phases, not zero;
- acquisition: floating-point voltage, current, power, energy, frequency, and status registers where available;
- integer current fallback: read `CurrentIntScale`; use the resulting explicit multiplier rather than assuming 0.01;
- writes: expert-gated, preceded by backup and followed by readback/diff.

## Evidence and provenance

- Legacy scaling excerpt preserved at `docs/evidence/wattnode-current-scaling-note.png`.
- Authoritative reference: *WattNode Module for Modbus (WND Series) Reference Manual*, document WND-M1-MB-Ref-1.10, documented firmware 1028.
- Authoritative register source: current WND Series Modbus register list from the manufacturer support page.

## Documentation still needed for a clone-ready bridge configuration

1. Exact WattNode model label, serial number, and firmware.
2. CT manufacturer/model, amp rating, output rating, and phase assignment for A/B/C.
3. Service type and voltage wiring used at the installation.
4. Full read-only WattNode configuration snapshot before changes.
5. Current bridge export plus firmware identity.
6. Selected float register set, word order, polling interval, and Loriot payload proof.
7. Any existing ExactAire portal multiplier values, captured as legacy evidence rather than treated as the source of truth.

## Bench validation sequence

1. Photograph meter and all three CT labels.
2. Read identity and firmware without writes.
3. Read `CtAmps1..3`, `CurrentIntScale`, power scaling, gain, phase, and nominal CT voltage settings.
4. Read float measurements and compare them with the local display or a trusted reference.
5. Select the smallest useful point set for the bridge.
6. Export the bridge baseline, compile the WattNode point configuration, write with confirmation, and read back.
7. Confirm join/uplink and decode the resulting Loriot raw payload.
8. Export a clone package containing the device profile, bridge configuration, firmware compatibility, and acceptance evidence.

## Sources

- [WattNode Module Modbus support and downloads](https://ctlsys.com/support/wattnode-module-modbus/)
- [WattNode Module for Modbus WND Series Reference Manual](https://ctlsys.com/wp-content/uploads/2017/10/WND-Module-Modbus-Ref-Manual.pdf)
