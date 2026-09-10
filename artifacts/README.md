# Build and review files

The latest verified beta is [Polygon Device Configurator](releases/Polygon-Device-Configurator-windows-x64-20260910-184316/). The matching ZIP and SHA-256 checksum are in releases/.

Older release folders, ZIPs, and checksums, including archived releases, were deleted at the user's request on 2026-09-10. Only this beta remains in the release folders. Extracted package-test copies were also removed; their logs and screenshots remain. Cleanup removed 118 entries totaling 885,805,077 bytes. The local deletion record is review/build-cleanup-latest-only.json.

Review logs and screenshots remain in review/. Rust build caches remain under native/modbus-configurator/target/. Source profiles, manuals, and hardware evidence are retained. These generated output directories are excluded from Git.

The retained beta predates the source-formatting and module-refactor pass. Build the current source with tools/Build-WindowsPackage.ps1 when a new executable is needed.
