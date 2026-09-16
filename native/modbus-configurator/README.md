# Polygon Device Configurator — Rust application

From the repository root:

```powershell
./tools/Check-Rust.ps1
./tools/Build-WindowsPackage.ps1
./tools/Test-WindowsPackage.ps1 -ZipPath artifacts/releases/<package>.zip
```

Check-Rust runs locked/offline MSVC Clippy with warnings denied and the complete instrumented test suite with an 90% line-coverage floor. See [coverage](../../docs/RUST-COVERAGE.md). Build-WindowsPackage uses target/portable-build, embeds resources, inspects imports and produces a timestamped folder/ZIP/checksum with licenses. Rust MSVC and llvm-tools-preview are development dependencies.

Launch Polygon Device Configurator.exe from the extracted package. Close previous instances first. No Python or PowerShell is needed at runtime. Hardware discovery uses USB metadata and protocol validation, not fixed COM numbers. Shared-bus direct adapter access requires the Modbus Bridge switched off.

The package smoke test runs the actual extracted executable with isolated application data and restricted PATH. --offline disables hardware; --capture-views writes real rendered views, and --exit-after-capture completes offline capture. Currently 34 views. No sample sensor data is injected by this mode.

[Operator guide](../../docs/OPERATOR-GUIDE.md) covers configuration, history and settings. [Current support](../../CURRENT-STATUS.md) distinguishes implementations from physical acceptance. Storage is `%LOCALAPPDATA%/Polygon/Device Configurator/` (Settings.json, Selected.tsv, Backups).

Development reference regeneration:

```powershell
python tools/export_native_reference.py
python tools/export_native_reference.py --check
```

Run these from repository root with the development Python environment. Reference generation includes reviewed native extensions. Device PDF mappings are in reference::manuals; extraction is an explicit embedded-path allowlist. Never add credential-bearing transcripts to release assets. Firmware updates and native IAQ workflows remain pending.
