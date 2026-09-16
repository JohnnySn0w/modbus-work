# Vaisala DPT146 Modbus Bridge clone checklist

Status: Modbus Modbus Bridge import, local reads, and Loriot payload validation complete. DSP integration is deferred until after the initial Modbus device POCs.

## Physical connection - check before configuration

> **Do not confuse connector I/II with CH1/CH2.**

- `CH1` and `CH2` on the transmitter's side label are analog measurement outputs.
- Connector **I** carries those analog outputs.
- Connector **II** carries RS-485/Modbus and is the required Modbus Bridge connection.
- Both sockets are 4-pin M8 A-coded connectors with the same conductor colors.

Connector II wiring:

| DPT146 conductor | Function | Synetica Modbus Bridge terminal |
|---|---|---|
| Brown | 15-28 VDC positive | 20-24 VDC positive distribution |
| Blue | Power ground / signal reference | Supply negative and RS485 `C` reference |
| White | RS-485 D0- | RS485 `B` |
| Black | RS-485 D1+ | RS485 `A` |

## Confirmed device communications

- Slave ID: 1
- Baud: 19200
- Data bits: 8
- Parity: None
- Stop bits: 2
- Protocol: Modbus RTU

## Before cloning

- [ ] Read the target bridge model and firmware before importing anything.
- [ ] Require model `ENL-MOD-32` and firmware `3.6` for this validated artifact; block and revalidate other firmware versions.
- [ ] Confirm the M8 cable is in connector II.
- [ ] Confirm 15-28 VDC between brown and blue.
- [ ] Confirm white to bridge B, black to bridge A, and blue/common reference to C.
- [ ] Import `vaisala-dpt146-validated.tsv` using `Configure Device > Import/Export > Import (Tab delimited)`; omit its header row and finish with an empty line.
- [ ] Apply the confirmed bridge-wide serial settings.
- [ ] Read all configured points locally and require `8/0 (OK/Exceptions)`.
- [ ] Validate a Loriot uplink and correct raw-payload decoding; defer DSP mapping during the POC phase.
- [ ] Save the bridge export, serial number, DevEUI, firmware, and validation report.
