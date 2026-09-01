# WattNode WND-M1-MB bench readiness

This is the go/no-go checklist for the first physical WND-M1-MB. The software artifacts are ready for a read-only bench test; the profile remains **to-test** until evidence from a real unit is saved.

## What to have on hand

- WND-M1-MB with an unobstructed front label
- its intended CTs, with every CT rating/output label readable
- documented electrical service and voltage/CT phase mapping
- isolated 6–24 Vdc or 12–24 Vac instrument power
- USB-COMi-TB adapter and three conductors for A−, B+, and common
- a qualified person and appropriate PPE/test equipment for any mains-voltage work
- Polygon/Synetica ENL-MOD-32 bridge only after direct meter validation

The meter accepts dangerous mains voltage at its measurement terminals. De-energize and follow the manufacturer's installation instructions. CT secondaries must be handled according to the CT manufacturer's safety instructions. This project checklist is not an electrical installation procedure.

## Before connecting software

1. Photograph the full label. Record model, serial number, manufacture date, and the complete `Opt` text.
2. Record each CT manufacturer, model, primary current rating, nominal output (normally 0.33333 Vac), conductor/phase, direction, and terminal assignment.
3. Record service type: wye, delta, branch circuits, house/inverter, line-to-line, or custom.
4. Record voltage input mapping and expected nominal voltages.
5. Confirm whether option `T1` is present. It installs termination and bias internally; do not add duplicate termination at a short bench endpoint.
6. Keep the ENL-MOD-32 and every other Modbus master disconnected for direct probing.

## Safe first contact

1. Wire USB-COMi `TxD−` to WattNode `A−`, `TxD+` to `B+`, and GND to common. In two-wire mode, use the adapter's paired data terminals as documented by its label.
2. Apply instrument power. Allow at least 300 ms for communications startup.
3. Use the label options if present. Otherwise try only these bounded candidates:
   - address 1, 19200 baud, 8N1
   - address 127, 19200 baud, 8N1 (legacy no-DIP firmware default)
4. Do not scan all 247 addresses on an operating bus and do not write communication registers.
5. Read Function 17 Report Slave ID, then diagnostic registers 1701–1708. Accept a family fingerprint only when:
   - the identity text names Continental Control Systems and WattNode;
   - model register 1707 equals `530`;
   - firmware register 1708 is in the WND module `10xx` family;
   - serial register 1701–1702 is nonzero and matches the label.
6. The exact `WND-M1-MB` model still comes from the physical label; the protocol identifies the meter-module family.

If energized from a measured AC service, integer frequency register 1221 (PDU address 1220) is a simple independent first measurement. It avoids float ordering and installation scaling. A normal US supply should be near 60 Hz. A zero reading may simply mean the voltage measurement inputs are not energized.

## Mandatory read-only snapshot

Save these before any configuration write:

- identity/diagnostics: 1701–1723
- factory options: 1724–1730 and 1737
- communications: 1652–1656
- CT configuration: 1603–1607
- calibration/configuration: 1608–1619
- scaling and mapping: 1622, 1624–1626, 1628–1637

The profile records both manual one-based register numbers and zero-based PDU addresses. Low-address 32-bit words come first: the lower-address register contains the least-significant 16-bit word. Read both words in one request.

## Configuration decision

Do not create a write plan until all of these are known:

- CT rated amps for channels 1–3 (`CtAmps1..3`, registers 1604–1606)
- CT nominal output voltage (`NominalCtVolts1..3`, registers 1629–1634)
- service mapping (`ConnectionType`, register 1636, and `MeterConfig1..3`, 1624–1626)
- whether factory option `L` or a passcode locks configuration
- whether the existing values already match the installation

Connection shortcuts documented by the manufacturer:

| ConnectionType | Use | MeterConfig 1/2/3 |
|---:|---|---|
| 1 | 3-phase 4-wire wye | 10 / 20 / 30 |
| 2 | 3-phase 3-wire delta | 90 / 50 / 0 |
| 3 | 1–3 neutral-connected branch circuits | 10 / 10 / 10 |
| 4 | house and inverter | 10 / 20 / 40 |
| 5 | single-phase line-to-line | 40 / 0 / 0 |

Never guess gain or phase adjustments. Preserve those values unless traceable CT characterization/calibration data says otherwise. Leave `CurrentIntScale` at its documented default unless a reviewed legacy integer pipeline requires another value. The supplied bridge POC uses native floats and does not depend on integer current/power scaling.

## Acceptance sequence

1. Save the label photos and the complete read-only snapshot.
2. Confirm identity, serial, firmware, communication settings, and option registers agree.
3. Compare CT and service configuration to the physical installation.
4. If changes are required, back up first, show a field-by-field diff, require confirmation, write the smallest possible set, then read back and diff.
5. Read the 12 native-float candidate points directly. Check finite/plausible values and low-word-first decoding.
6. Compare voltage, current, power, and energy behavior with a trusted reference under a known load.
7. Substitute the confirmed slave ID in `artifacts/bridge-config/wattnode-wnd-m1-mb-documentation-test.tsv`.
8. Back up the ENL-MOD-32, import/write the candidate, then export/read back and diff.
9. Confirm the corresponding Loriot payload changes plausibly with load.
10. Attach evidence and promote the profile only after review.

## Current readiness verdict

Ready now: register map, address conventions, 12-point bridge candidate, deterministic family fingerprint, legacy address fallback, snapshot scope, CT/service decision rules, and acceptance sequence.

Required from the physical installation: exact label/options, CT labels, service/wiring map, live firmware, initial snapshot, trusted measurement comparison, bridge readback, and Loriot capture.

## Manufacturer references

- [WND-M1-MB reference manual, revision 1.10](https://ctlsys.com/wp-content/uploads/2017/10/WND-Module-Modbus-Ref-Manual.pdf)
- [WattNode Module support page](https://ctlsys.com/support/wattnode-module-modbus/)
- [WND option identification](https://ctlsys.com/support/wattnode_modbus_option_identification/)
- [WND firmware versions](https://ctlsys.com/support/wnd-wattnode-modbus-firmware/)
- [WattNode Modbus FAQ](https://ctlsys.com/support/wattnode_modbus_faq/)
