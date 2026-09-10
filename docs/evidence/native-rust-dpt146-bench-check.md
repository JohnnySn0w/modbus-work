> Historical evidence: observations and software state at the recorded test date. Earlier names, COM numbers, tool behavior and acceptance claims are preserved as evidence, not current instructions. See [current status](../../CURRENT-STATUS.md).

# Native Rust DPT146 bench check — 2026-09-09

Expected physical setup, reported by the operator: ENL-MOD-32 bridge with a
Vaisala DPT146 attached. The initial attempts did not independently verify identity.

## Initial observations

- The earlier Rust GUI was already running and was left open. Computer-use access
  timed out; no UI commands were performed.
- The Rust service enumerated COM5, USB VID/PID 0483:5740, USB serial
  `00000000001A`, described by Windows as USB Serial Device (COM5). USB metadata
  alone cannot distinguish the bridge from other Synetica products.
- Windows allowed the diagnostic harness to open COM5 at 115200 8N1, DTR enabled,
  RTS disabled. There was no port-ownership conflict.
- Initial passive collection timed out after five seconds without received bytes.
  A second attempt used one bounded carriage-return wake and timed out before a
  recognized console prompt. Both failed sessions were closed.
- Neither attempt reached model/firmware verification, authentication, table
  export, or bridge Read All. No configuration/import or direct RS-485 commands
  were issued. The only console input was one carriage return on the second attempt.

## Operator clarification

The operator reports that this unit needs its USB connection physically reconnected
after each successful serial connection. Treat that as a bench prerequisite, not
as a firmware property proven by these failed attempts. Export and Read All must
share a single persistent port handle. Closing the session or ending the harness
means another physical USB reconnect is needed before the next connection.

The operator reconnected USB and a third attempt received console bytes, but no
recognized prompt was found before the response deadline. It did not send the
silent-console wake because bytes had arrived. No login/export/read commands ran.

The operator explicitly authorized exploring Windows driver/kernel-level USB
reconnection. Windows identified this exact device instance:
`USB\VID_0483&PID_5740\00000000001A`. A non-administrator PnPUtil restart attempt
reported Access denied. The elevated restart request ended with Windows reporting that the operation was cancelled by the user;
software reset has not yet been verified. No other device/hub was targeted.

Preserve the DPT146 point table, external power, and RS-485 wiring. After the
restart completes, capture one fresh connection with the private diagnostic
harness to determine the actual prompt syntax; previous failed responses were
not recorded as raw transcripts.

## Diagnostic harness

This uses the same Rust service and per-port worker as the GUI, not PowerShell:

```powershell
cargo +stable-x86_64-pc-windows-msvc run --locked --manifest-path native/modbus-configurator/Cargo.toml --example bench_check -- inventory
cargo +stable-x86_64-pc-windows-msvc run --locked --manifest-path native/modbus-configurator/Cargo.toml --example bench_check -- read-all COM5
```

Inventory does not open the port. `read-all` performs identity/login, native table
export, and Read All in one connection. On completion the harness reports JSON,
checks table equality with the DPT146 catalog, then closes the connection. Table
equality alone is not physical instrument identification.

`examples/capture_read.rs` can save raw receive bytes during the same core
read-only state-machine workflow. Such a trace may contain credentials and must
go to a new file in `.secrets` or another approved private location.

## Captured protocol findings — 2026-09-09 follow-up

### Latest prepared-sequence test

**Subsequent success:** Two later runs each completed native backup, a complete
Detailed read, and a repeat read without another physical USB reconnect between
the runs. The second used the integrated production transport. It reported
temperature 23.793625 °C, dew/frost point 8.576037 °C, atmospheric dew/frost point
8.600575 °C, moisture 11150.516602 ppmv, pressure 1.011405 bara, fault status 1,
online status 1 and error code 0. Each read also reported exception 2 for points
9–12 and matched the bridge's 8/4 summary. This is a complete mixed result, not
an 8/0 commissioning pass.

The transport reopens immediately after a command and resumes stalled partial
replies at the quiet interval, retaining received bytes without resending input.
Recovery is bounded by a 90-second deadline and 32 reopens per command. DTR/RTS
are lowered before closing. One session owner retains identity while the OS
handle is reopened. The earlier five-second wait lost output. The exact driver
or firmware cause remains unproven; one experimental run had a transport error.

`tests/fixtures/bridge-detailed-complete.txt` contains the sanitized successful
repeat transcript. Tests verify explicit readings, exceptions, retries, complete
point coverage and summary agreement. The application shares this transport and
parser. Upload remains unimplemented; the twelve-point table was not changed.

The following paragraphs record the earlier failed sequence for context:

The first attempt opened COM5 but received no bytes and timed out after one wake.
After explicit DTR-low/RTS-low cleanup, the next attempt received the bridge
banner immediately, logged in and completed native export. This observation is
consistent with close/reopen recovery but does not isolate its cause or establish
reliability; no Windows device reset was performed.

The saved, live backup is `bridge-live-12-point-backup-2026-09-09.tsv` beside this
document. It contains **12 points**, not eight. Points 1–8 match the validated
DPT146 configuration; extra points 9–12 read PDU addresses 1032, 1162, 1164 and
1166 as F32/HL. This supersedes assumptions about the current configured count.

The same session owner then entered Read All and selected Detailed. Received
output includes Modbus exception 2, Illegal Data Address, for points 9–12,
including retries. The captured detailed reply is incomplete at its beginning,
so this run does not establish all eight expected measurement values. The
application rejected the read and did not execute the repeat. It did not upload,
delete or alter any configuration rows. The regression fixture contains only
the observed read-options/measurement tail, not identity or credentials.

Native backup-to-file is now demonstrated through the diagnostic harness.
The GUI save dialog, successful complete measurement decoding, upload/readback
and repeated successful reads remain unverified.

Private receive traces now exist under the ignored native `target` directory.
They are not fixtures for redistribution. The earlier inference that a timeout
meant no login ran was too strong: the new capture observed a password banner,
followed on the next open by masked password echo and a main menu.

After the operator reconnected USB, the instrument returned the initial banner
immediately. Subsequent replies arrived only after the host handle was reopened:
main menu at about 5.3 s, Modbus menu at 10.6 s, import/export at 15.9 s, native
eight-row export at 21.2 s, import/export at 26.5 s and Modbus menu at 31.8 s.
The native adapter now performs one bounded host reopen after a silent command
timeout, preserving the active identity and receiving without resending input.
This is distinct from a Windows device reset or external power cycle.

Read All did not complete. A subsequent receive captured the actual intermediate
`Read All Data Points Options` menu with `N - Normal`, `D - Detailed` and default
Normal. Both the Python helper and original Rust replay skipped this screen.
Rust now selects Detailed when this prompt appears. The initial wait before host
reopen is bounded by the normal response timeout, including Read All; the overall
operation limit is 90 s. Tests cover this menu and no-resend reconnect behavior.

The eight-row export was parsed during the live run, but end-to-end backup/save,
Read All, upload and exported readback are not yet accepted. No configuration
rows were uploaded or deleted in these attempts. Another operator USB reconnect
was requested to test the complete corrected read-only workflow.

Microsoft references for the reset investigation:
[PnPUtil restart-device](https://learn.microsoft.com/en-us/windows-hardware/drivers/devtest/pnputil-command-syntax)
and [USB hub port cycling](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/usbioctl/ni-usbioctl-ioctl_usb_hub_cycle_port).
