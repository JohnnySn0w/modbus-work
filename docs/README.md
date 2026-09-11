# Documentation index

Start with the operator guide for application use. Device instructions without established support are marked **TBD**. Historical records describe individual tests, not general installation requirements.

- [CI/CD](CI-CD.md): Windows checks, build artifacts, and tagged beta releases.
- [Code style](CODE-STYLE.md): formatting, module size, and concise documentation.
- [Operator guide](OPERATOR-GUIDE.md): use, settings, data retention and configuration.
- [Current status](../CURRENT-STATUS.md): support matrix and acceptance boundaries.
- [Goals](../GOALS.md): consolidated unfinished work.
- [Architecture](ARCHITECTURE.md), [native build guide](../native/modbus-configurator/README.md), [parity](RUST-PARITY.md), [coverage](RUST-COVERAGE.md).
- [UI structure](UI-INFORMATION-ARCHITECTURE.md), [configuration audit resolution](CONFIGURATION-UI-AUDIT.md), [design decisions](DESIGN-DECISIONS.md).
- [Manual inventory](reference/README.md), [device artwork](DEVICE-ARTWORK.md).
- [Firmware design](FIRMWARE-UPDATES.md), [HMD65 representations](devices/hmd65-representation-review.md).
- [WattNode verification status](WATTNODE-TEST-READINESS.md), [remaining register research](REMAINING-DEVICE-REGISTER-ASSESSMENT.md), [profile lifecycle](../artifacts/device-profiles/PROFILE-LIFECYCLE.md).

## Evidence and history

Files in evidence/ and dated validation records retain original observations, including former names, test counts and observed COM ports. They are not live hardware status. Source PDFs and register artifacts are not rewritten by a documentation pass. Previous versions of updated guides are retained in archive/2026-09-10-before-documentation-pass, locally only, outside version control.

The documentation review inventory is documentation-review.json. Documentation edits do not update existing packaged executables. Build a new package to include interface wording changes.
