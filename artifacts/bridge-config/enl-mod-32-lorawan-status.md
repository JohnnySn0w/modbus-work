# ENL-MOD-32 LoRaWAN status

Observed read-only on 2026-08-28 after local DPT146 configuration validation.

## Join and reporting

- Status: Joined
- Region: North American hybrid 915 MHz
- Network mode: Public network ON
- Activation credentials: provisioned; key values intentionally excluded
- Transmit interval: 15 minutes
- Uplink port: 1
- Receive port: All
- Join-check interval: 3 hours
- Message confirmation: OFF (unconfirmed uplinks)
- ADR: ON
- Current/initial displayed data rate: DR0, SF10, BW125
- Transmit power: 20 dBm
- Duty-cycle enforcement: OFF

## Last observed radio metrics

- RX RSSI: -44 dBm
- RX SNR: 27 dB
- RX count: 1
- TX count: 18
- Last TX time: 370 ms

These metrics indicate a strong local radio link. A subsequent Loriot capture confirmed the corrected eight-point payload; see `enl-mod-32-lorawan-payload.md`. Downstream DSP work is deferred until after the initial Modbus device POCs.

## Payload composition

- All optional bridge KPI values were OFF and therefore excluded from normal radio packets.
- The corrected eight Modbus points are enabled for interval reads.
- The bridge reported 8 successful Modbus reads and 0 exceptions before the LoRaWAN inspection.

## Completed POC acceptance evidence

- [Complete] Capture an uplink after the corrected table was installed.
- [Complete] Record frame counter, timestamp, port, and raw payload.
- [Complete] Identify Loriot as the network server.
- [Complete] Decode and semantically map all eight payload records.
- [Deferred] DSP routing, field names, units, precision, and application semantics.
- Preserve the provisioned application key; no rotation is planned for this unit.
- Keep credentials in the local Git-excluded secrets record, not in clone tables or technician-facing documents.
