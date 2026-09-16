# Native application implementation

The delivered executable is **Polygon Device Configurator.exe**. Cargo's internal binary name remains modbus-configurator.exe. Rust/egui UI and serial service run in one process using typed commands/events.

[Architecture](ARCHITECTURE.md) describes implemented boundaries. [Operator guide](OPERATOR-GUIDE.md) describes behavior. [Build guide](../native/modbus-configurator/README.md) provides reproducible commands. [Parity](RUST-PARITY.md) and [goals](../GOALS.md) list unfinished work.

Modbus Bridge sessions, DPT146 adapter acquisition, verified table programming and local file workflows are implemented. Native IAQ sessions, firmware image inspection/flashing, and complete multi-product Synetica support are not. Runtime state is never sourced from Python snapshots.

The package embeds fonts, device photos, SVG-derived icon raster resources, profiles and seven PDF manuals. It uses static C runtime linkage and package dependency checks. USB drivers remain host dependencies. A successful package check on this PC is not clean-PC qualification.
