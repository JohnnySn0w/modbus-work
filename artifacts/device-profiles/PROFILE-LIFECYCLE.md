# Device profile lifecycle

Every supported or candidate Modbus instrument uses the same profile structure and evidence gates. Lack of bench hardware does not prevent documentation work, but it does prevent deployment status.

## Statuses

### `to-test`

Documentation-backed hypothesis for a device that is not presently available.

- May contain authoritative register definitions, candidate serial settings, wiring notes, and a proposed Modbus Bridge point set.
- Unknowns remain explicit as `null`, `unresolved`, or preflight blockers.
- Must set `deployment_allowed: false` and `golden_configuration: false`.
- Must distinguish safe reads, installation writes, calibration-only controls, and hazardous actions.
- Not qualified for field deployment. The Rust configuration selector exposes documentation candidates for controlled bench testing; software selection is not a deployment approval.

### `bench-validated`

Tested against an identified physical instrument.

- Exact model, device firmware, serial number, manual/register-list revision, and test date are recorded.
- Serial settings, addressing convention, types, byte/word order, scaling, units, and sentinel values have live evidence.
- Selected measurements have been compared with plausible conditions, a display, simulator, or trusted reference.
- Writable fields have readback evidence; unsafe/calibration fields remain guarded.

### `bridge-validated`

The device profile has been compiled into an identified Modbus Bridge and tested locally.

- Exact Modbus Bridge model and firmware are recorded.
- Baseline backup, configuration diff, write/import, readback, and rollback path are available.
- All configured points pass the Modbus Bridge's local read test.

### `clone-ready`

Approved golden package for technician deployment.

- LoRaWAN join and correctly decoded Loriot raw payload are proven.
- Exact firmware compatibility policy is recorded.
- Package includes the device profile, Modbus Bridge configuration, manifest, wiring notes, screenshots/log evidence, acceptance checklist, and secrets handling instructions.
- DSP routing after Loriot is outside the current POC acceptance boundary.

## Promotion rule

Profiles only move forward. Documentation confidence cannot substitute for physical validation. A later manual or firmware change can return a profile to `to-test` or block deployment until revalidated.

## Backlog policy

For devices not currently on hand:

1. obtain authoritative manuals and register maps;
2. create a `to-test` profile from the shared template;
3. record unresolved documentation and required physical evidence;
4. propose a minimal useful float-first measurement set;
5. prepare a safe, read-only bench sequence;
6. wait for hardware before assigning any validated or clone-ready status.

