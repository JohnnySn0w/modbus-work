# Build and review files

Published Windows betas are on the [GitHub releases page](https://github.com/JohnnySn0w/modbus-work/releases). Each release includes the executable and a portable package with matching debug symbols.

For the latest local package, use the root **Polygon Device Configurator** shortcut or `tools/Start-LatestBuild.ps1`. The launcher selects the latest verified build metadata instead of relying on a fixed folder name. Run it with `-ResolveOnly` to print the executable path.

`releases/` contains generated package folders, ZIP files and checksums. `review/` contains package-check logs and screenshots. Rust build caches remain under `native/modbus-configurator/target/`. These generated directories are excluded from Git; source profiles and public documentation remain tracked.

Build current source with `tools/Build-WindowsPackage.ps1`; validate a package with `tools/Test-WindowsPackage.ps1 -ZipPath <package.zip>`. Local builds are independent of GitHub releases, which are created by the tagged CI workflow.
