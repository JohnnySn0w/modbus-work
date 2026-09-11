Polygon Device Configurator — Windows x64 portable build

Extract the ZIP into a folder and double-click Polygon Device Configurator.exe.
No Python, PowerShell, Rust installation or separate Visual C++ runtime is
required by this executable. It uses Windows system DLLs and the graphics driver.
No installer or administrator access is required to launch the app.

Normal launch automatically discovers supported USB interfaces and reads live
values. Close other programs using the same serial interface before connecting.
COM port numbers are discovered dynamically; they are not installation settings.

To browse without accessing hardware, launch "Polygon Device Configurator.exe" --offline.
Offline mode shows no connected devices and generates no sample readings.
To return to hardware operation, close that instance and launch normally.

The executable embeds reference tables, device photographs and Brandon Text fonts.
Open reference documents through References; no repository checkout is needed.
Documents open using the Windows application associated with their file type.

Settings and point-table backups are stored under:
  %LOCALAPPDATA%\Polygon\Device Configurator
History lasts for the application session. Export a comma-separated values file
before closing to retain samples. Replacing the application folder does not
delete saved settings or backups.

Current support:
- E5 bridge firmware 3.6 and Vaisala DPT146 readings, programming and restore tested.
- HMD65/WattNode presets and direct USB adapter reads implemented; hardware
  qualification remains pending.
- IAQ is reference-only. Firmware flashing and radio writes are unavailable.
- Full power-loss persistence and extended soak acceptance remain pending.

This is a development release. See build-info.json for the executable hash and
Windows DLL imports, and DEPENDENCIES.txt/licenses for dependency notices.
Device photographs and Brandon Text fonts are bundled with the application.
