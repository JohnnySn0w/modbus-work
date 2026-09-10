# Polygon Device Configurator

Windows desktop application for live sensor readings, E5 bridge point-table configuration and USB Modbus device access. Built in Rust; Python's older staged displays are not runtime data.

Start with the [operator guide](docs/OPERATOR-GUIDE.md), [current support/status](CURRENT-STATUS.md), and [goals](GOALS.md). Download/launch the current package listed in [artifacts](artifacts/README.md). Close the previous application before launching a replacement.

Supported code paths cover E5 bridge firmware 3.6, DPT146, HMD65, WattNode WND-M1-MB and an ATI F12/PAA E5 profile. Hardware acceptance differs by device; IAQ Plus native readings and all firmware updating remain unfinished.

Development: [native application](native/modbus-configurator/README.md), [architecture](docs/ARCHITECTURE.md), [parity](docs/RUST-PARITY.md), [coverage](docs/RUST-COVERAGE.md). [Documentation index](docs/README.md) distinguishes current guides from historical evidence.
