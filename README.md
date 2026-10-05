# Polygon Device Configurator

Windows desktop application for live sensor readings, Modbus Bridge point-table configuration and USB Modbus device access. Readings come from connected hardware.

Start with the [operator guide](docs/OPERATOR-GUIDE.md), [current support/status](CURRENT-STATUS.md), and [goals](GOALS.md). Download/launch the current package listed in [artifacts](artifacts/README.md). Close the previous application before launching a replacement.

Supported code paths cover Modbus Bridge firmware 3.6, DPT146, HMD65, WattNode WND-M1-MB and an ATI F12/PAA Modbus Bridge profile. Hardware verification differs by device. HMD65 non-metric is selectable; metric HMD65 and IAQ sections are hidden. Firmware updating is not implemented.

Development: [native application](native/modbus-configurator/README.md), [architecture](docs/ARCHITECTURE.md), [parity](docs/RUST-PARITY.md), [coverage](docs/RUST-COVERAGE.md). [DSP setup guide](How%20do%20configure%20Modbus%20Bridge%20on%20DSP.md). [Documentation index](docs/README.md) distinguishes current guides from historical evidence.
