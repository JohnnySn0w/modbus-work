> Historical evidence: observations and software state at the recorded test date. Earlier names, COM numbers, tool behavior and acceptance claims are preserved as evidence, not current instructions. See [current status](../../CURRENT-STATUS.md).

# Portable Windows package acceptance — 2026-09-10

Built `artifacts/archive/releases/ExactAire-Modbus-windows-x64-20260910-142822.zip`
(5,821,401 bytes), with ExactAire.exe, README, dependency notices/licenses and
build-info.json. SHA-256 checksum file accompanies the ZIP.

The previous release imported VCRUNTIME140.dll. The package build adds
`-C target-feature=+crt-static` for the explicit x86_64-pc-windows-msvc target.
llvm-readobj now reports Windows system DLL imports only: no VCRUNTIME, MSVCP,
Python or PowerShell runtime dependency. The build uses a separate target tree,
so it does not overwrite the running application.

Executable SHA-256:
`14bd44488eaafbd73abde31e53fd0a0b4e2866eb245121f63254734a3c2b19dc`

ZIP SHA-256:
`bb662df35892e46729318ad94c89620bdc69cfd0e2f7f78e44c56bd5ef624c43`

## Launch acceptance

`tools/Test-WindowsPackage.ps1` extracted the ZIP into a fresh workspace folder,
verified both hashes, and launched ExactAire.exe from that extracted folder.
PATH contained only C:\WINDOWS\System32; Python, python3, PowerShell, pwsh,
Cargo and rustc executable commands were confirmed unavailable. LocalAppData and
temporary directories were redirected to fresh test directories. The build and
test scripts use PowerShell on the developer machine; the application does not.

The app ran with `--offline --capture-views <new-folder> --exit-after-capture`.
The offline service has an empty inventory and rejects hardware operations.
All 18 rendered PNGs were produced and validated; the process exited with code 0.
The Devices and Configuration captures were visually checked: explicit offline
label, empty real-device list, bundled preset data, disabled programming action.
This did not touch the live bridge session or existing app settings.

Evidence: `artifacts/review/package-check-aa6627a5e1ce468ea2225b347d3ee6ad/result.json`
and its `views/` directory. The package has the preceding GUI event-handler fixes.
102 tests and all-target warnings-denied Clippy pass. A fresh machine/VM and
physical adapter qualification remain pending; this test does not claim those.

## Repeat

```powershell
./tools/Build-WindowsPackage.ps1
./tools/Test-WindowsPackage.ps1 -ZipPath <generated-zip>
```

Use the normal launch (without --offline) for real USB discovery. Close the old
configurator before launching another hardware-enabled instance. History still
lasts for the app session, so export it before closing if it needs to be retained.


## Recovery release check - 2026-09-10

Built artifacts/archive/releases/ExactAire-Modbus-windows-x64-20260910-145912.zip with
the coverage seams and USB handoff/backup discovery fixes. The extracted package
passed its isolated offline launch check, captured 18 views and exited with code 0.
Only System32 was on its runtime PATH; the live application was not restarted.
Result: artifacts/review/package-check-a1843cf167d4432babeda1cad3728143/result.json.
ZIP SHA-256: e9076a48af1ba241a551d7c8cbc35a2a4c42169383082fc432b285fa82942514.
This is current-host acceptance, not a second-PC or hardware qualification.


History usability build: artifacts/archive/releases/ExactAire-Modbus-windows-x64-20260910-150351.zip.
The isolated offline package check passed with 18 captured views and exit
code 0. Evidence: artifacts/review/package-check-9a0f6ee5b08948c4a9c270a803d59b85/result.json.
ZIP SHA-256: 14953ac8b0c5c29f936608f8bf72bfcd28cbcd47436124c152c971756cd5856d.
The existing live GUI has not been restarted.


## Polygon product review changes - 2026-09-10

- Renamed the native UI, executable, package and Windows version metadata to
  Polygon Device Configurator. The bridge reference uses Synetica ENL-MOD-32.
- Embedded the official Polygon ICO resource and PNG window icon. Asset provenance
  is recorded in docs/branding/README.md. Legacy storage is read only for migration;
  saved selections/backups are copied into LOCALAPPDATA/Polygon/Device Configurator
  without replacing newer files. Historical evidence retains historical names.
- Added a transfer progress bar: acknowledged row counts during delete/import;
  indeterminate animation during backup download and other unknown-length phases.
- Equal-width reference cards use wrapping text; geometry checked at 960 and 1180
  point window widths and visually checked in the packaged application.
- Device list and configured-device detail warn on sensor-profile changes, failed
  reads and implausible readings. Configured models are explicitly unverified.
  The bridge does not provide a dependable attached-sensor model identity: these
  are possible-mismatch warnings, not proof of the physical sensor model.
- Added per-register cycling of compatible display units on device/readout and
  register pages. Supports Celsius/Fahrenheit/Kelvin, absolute pressure, concentration,
  mass ratios/density, energy, power, voltage, current and frequency. Display choices
  share state across those pages for the session; native TSV/history/values stay intact.
  Relative humidity and status/enum registers receive no inappropriate conversions.
- Clear old per-register receipt times when the native point table changes.

130 tests pass; warnings-denied Clippy passes; overall line coverage is 90.02%.
Python reference generation --check also passes. The new executable contains ICON,
GROUP_ICON and VERSIONINFO resources. An isolated offline package launch captured
18 views and exited normally under normal Windows permissions. The sandbox could
not launch the deeply extracted executable, while the same check outside it passed.
No bridge writes or reads were performed for this review pass.

Package: artifacts/archive/releases/Polygon-Device-Configurator-windows-x64-20260910-154422.zip
Acceptance: artifacts/review/package-check-d9fd0dc4b44941cdb7fdaf34b704e86d/result.json
ZIP SHA-256: eb9800b6df64d0d6a87bab228c7d336c5d5b21677bb64ebabc85cb2f1e9097bb
The existing live instance was not restarted; use the new executable for review.

### Fixed status bar build

Release: `artifacts/archive/releases/Polygon-Device-Configurator-windows-x64-20260910-160354.zip`. Offline package verification passed under normal Windows permissions with 18 view captures. Report: `artifacts/review/package-check-3e426cc18a53439cbd5b050abed152a3/result.json`. Visually inspected References: persistent footer separated from scrollable content. No hardware interaction.
