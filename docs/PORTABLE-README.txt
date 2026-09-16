Polygon Device Configurator — Windows x64 portable build

Extract the ZIP into a folder and double-click Polygon Device Configurator.exe.
The window title shows the release tag or local build identifier so copies can
be distinguished even when the executable has been renamed or moved.
No Python, PowerShell, Rust installation or separate Visual C++ runtime is
required by this executable. It uses Windows system DLLs and the graphics driver.
No installer or administrator access is required to launch the app.

Release builds include full debug symbols, debug assertions and integer overflow
checks. Keep modbus_configurator.pdb with this exact executable when debugging;
symbols from another build cannot be substituted. The ZIP includes both files.
The executable remains optimized, so some variables or frames may be unavailable
in a debugger. Symbols do not automatically record crashes or create crash dumps.

For troubleshooting, use Diagnostics > Activity log > Export diagnostics.
This saves retained timestamped logs, build identification, background operation
summaries and Rust panic backtraces in one text file. Logs persist under the
application data folder in Logs; up to ten process logs and their rotation files
are retained. Native crashes and forced termination may leave no final entry.

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
