# Documentation

## Operate the application

- [Operator guide](OPERATOR-GUIDE.md): connect, read, configure, back up and export.
- [DSP measurement setup](../How%20do%20configure%20Modbus%20Bridge%20on%20DSP.md): labels and units for complete single-device profiles. Original screenshot attachments are not bundled.
- [Device support](../CURRENT-STATUS.md): availability, hardware evidence and remaining verification.
- [Manufacturer manuals](reference/README.md) and [terminology](TERMINOLOGY.md).

## Configure or diagnose a network

- [Network configuration](MULTI-DEVICE-CONFIGURATION.md): slave addresses, register selection and custom profiles.
- [Serial line configuration](E5-LINE-CONFIGURATION.md): shared bus settings, separate from point tables.
- [Diagnostics](DIAGNOSTICS.md): communication checks, time limits and support logs.

## Build and maintain

- [Native build guide](../native/modbus-configurator/README.md), [release pipeline](CI-CD.md), [build artifacts](../artifacts/README.md).
- [Architecture](ARCHITECTURE.md), [code style](CODE-STYLE.md), [coverage](RUST-COVERAGE.md), [Python parity](RUST-PARITY.md).
- [Interface information architecture](UI-INFORMATION-ARCHITECTURE.md), [visual vocabulary](DESIGN-VOCABULARY.md), [design decisions](DESIGN-DECISIONS.md), [configuration controls](CONFIGURATION-UI-AUDIT.md), [artwork](DEVICE-ARTWORK.md).

## Plan and verify

- [Outstanding work](../GOALS.md), [product scope](../PROJECT.md), [firmware design](FIRMWARE-UPDATES.md).
- [HMD65 register research](devices/hmd65-representation-review.md), [WattNode verification](WATTNODE-TEST-READINESS.md), [remaining register research](REMAINING-DEVICE-REGISTER-ASSESSMENT.md), [profile lifecycle](../artifacts/device-profiles/PROFILE-LIFECYCLE.md).

## Evidence and history

Dated records and files under `evidence/` describe specific observations, not live connectivity or current interface behavior. Manufacturer PDFs and raw register artifacts remain source references. Legacy metric HMD65 definitions are retained for existing tables; their presence in the repository does not make them selectable.

[documentation-review.json](documentation-review.json) records review dates per file. Locally archived drafts and private firmware instructions remain outside version control. Interface wording changes require a rebuilt executable.
