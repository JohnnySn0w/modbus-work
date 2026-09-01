# Firmware update architecture

Firmware updates are a separate high-risk commissioning workflow. They are not configuration writes and must never be implemented as an unrestricted file picker followed by a flash command.

## Current evidence boundary

The attached IAQ Plus identifies as `FW-AQ-VCP+` version 5.06. Synetica's public IAQ material documents USB configuration and reboot after LoRaWAN key changes, but the public downloads reviewed do not publish an IAQ Plus firmware image format, bootloader-entry sequence, flashing utility, upgrade matrix, or failed-flash recovery procedure. The updater therefore remains blocked pending vendor material.

## Required firmware package

Every approved image must be stored outside or alongside the repository with a versioned `manifest.json` derived from `artifacts/firmware/manifest-template.json`. The manifest binds:

- exact product family and firmware code;
- target version and allowed source versions;
- allowed radio regions;
- image filename and SHA-256;
- vendor approval/source and release notes;
- exact flashing method;
- recovery procedure;
- post-flash acceptance checks.

The application validates this metadata and hash before it can offer an update. A filename or version string alone is never sufficient.

## Technician workflow

1. Identify exact model, firmware code/version, hardware revision when available, and radio region.
2. Read and save the complete private configuration and credential backup.
3. Confirm the device is externally powered and USB is stable; block laptop sleep for the operation.
4. Select a vendor-approved package and validate compatibility plus SHA-256.
5. Show release notes, upgrade path, expected USB re-enumeration, and recovery instructions.
6. Require an explicit confirmation naming the source and target versions.
7. Enter the vendor-documented bootloader/update mode.
8. Flash through an adapter dedicated to that vendor method; stream progress and preserve raw updater logs.
9. Wait for normal USB re-enumeration and re-identify the device from its banner.
10. Verify target firmware, region, JoinEUI preservation, AppKey state, and configuration readback.
11. Restore only settings that the release procedure says are not retained.
12. Confirm Loriot join and an advancing uplink frame counter; measurements are optional for IAQ credential commissioning.
13. Produce an immutable update report containing hashes, versions, timestamps, result, and recovery actions.

## Failure and recovery rules

- Never unplug or power-cycle solely because progress appears slow; obey the updater's documented timeout.
- If normal firmware does not re-enumerate, look only for the documented bootloader VID/PID.
- Never retry a different image as an experiment.
- Preserve the failed updater log and original backup.
- Recovery is allowed only with the vendor procedure bound to the selected package.
- A recovered device must repeat all post-flash checks before returning to service.

## Implementation state

Implemented:

- firmware manifest schema;
- image path containment and SHA-256 validation;
- exact product, firmware-code, source-version, and region gates;
- vendor-approval and recovery-procedure requirements.

Blocked:

- actual flash transport;
- bootloader identity and entry method;
- authoritative IAQ Plus image packages and release notes;
- recovery procedure;
- confirmation of configuration/key retention across update.

These blockers require a Synetica firmware package/updater and vendor instructions, or a controlled vendor-supported bench update. They cannot be safely inferred from the configuration console.
