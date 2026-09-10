# Current design decisions

- Windows-first native Rust deliverable; Python retained as development/reference prototype.
- Real acquisitions only. Retained values are labeled/timestamped and are not fresh history samples.
- COM numbers are routes, never identities; USB VID/PID alone does not identify a downstream sensor.
- One active master on shared RS-485. E5 hardware switch-off removes its USB interface while external power may remain.
- Persistent E5 serial ownership and bounded recovery; never blindly resend mutating commands.
- Reviewed point-table writes require durable backup and exported verification. No automatic rollback.
- Quiet automatic polling; explicit actions queue and use a fixed status bar.
- Display-unit settings are independent from native data and radio region. System theme/units are defaults.
- Product references and PDF manuals do not imply physical qualification.
- E5 point-table backup is not full configuration/credential/firmware backup.
- Native IAQ and firmware updating remain separate development work. Supplied bootloader instructions establish a route, not verified automation or recovery.

See [goals](../GOALS.md) for unresolved decisions/acceptance and [architecture](ARCHITECTURE.md) for source boundaries.
