# Diagnostics and communication checks

For normal use, see the [operator guide](OPERATOR-GUIDE.md). A successful USB connection, a verified point table and successful sensor readings are separate states.

## Isolate a connection failure

For a connection failure, turn automatic polling off, use **Read configuration only** in Diagnostics to check console access without polling instruments, then use **Read now** for the sensor read. **Live communication timeline** shows response deadlines, first-byte latency, byte counts, last-byte age, parser state, and bounded receive-recovery attempts. Read All also records configured entry count, slave addresses, and downstream settings when available. A successful recovery call is not proof that data resumed. No passwords or response payloads are recorded. **Copy log**, **Save log…**, and **Export diagnostics…** include the timeline. Use Export diagnostics for the longer retained history.

Selecting **Use this interface** in Diagnostics turns off automatic polling for the current session. Manual actions remain available. Enable **Automatic polling** in Settings to resume automatic readings. Selecting an interface does not change the saved startup preference.

## Assess register data

**Communication and data checks** assesses the last response before retained values are added to the display. Each snapshot names its interface and timestamp; it is not a live connection indicator. Checks compare each slave's full or partial register layout against reviewed profiles. A consistent address difference of one is reported as a possible indexing mismatch, never automatically corrected. The reviewed HMD65 Modbus Bridge convention is preserved.

Value checks flag non-finite numbers, broad physical bounds (such as relative humidity outside 0–100%), suspicious subnormal floating-point values, and defined fault/status values. These are plausibility checks, not accuracy checks or complete manufacturer operating limits. Zero alone is valid. Unknown or ambiguous profiles cannot receive device-specific interpretation. Incorrect indexing and word order can still produce plausible values.

Serial checks show the reported Modbus Bridge settings, flag incompatible data-bit settings, and compare the timeout against an estimated request/response transmission time. Instrument baud rate and parity cannot be inferred from a timeout or from a successful USB console connection. No automatic baud scan, alternate-address reads, or configuration writes are performed. Findings are included in exported diagnostics.

Assessment findings also appear in Activity log after manual reads or when automatic findings change. **Copy log** and **Save log…** include the complete latest assessment even if the visible activity list has been cleared or its oldest entries trimmed. **Export diagnostics…** includes assessment history across retained sessions.

## Adjust application time limits

**Settings → Advanced communication settings** controls application waits, separately from the Modbus Bridge's sensor settings. Changes apply to the next operation without reopening the active connection.

| Limit | Default | Scope |
|---|---:|---|
| Initial console response | 20 seconds | Initial response; a silent connection can also use one wake-response wait. |
| Menu and command response | 5 seconds | Individual console replies. |
| Read All response | 180 seconds | Minimum per phase when extra scan time is enabled. Read All can have two phases. |
| Automatic retry delay | 30 seconds | Recoverable failures; console timeouts and transport failures instead pause automatic polling. |

**Allow extra scan time for sensor retries** is enabled by default. It allows at least 15 seconds per configured entry plus 30 seconds, or more for longer reported sensor retries, capped at one hour per phase. Disable it to test the exact configured Read All deadline.

Longer waits do not establish that a scan is progressing. Check the communication timeline for received bytes and the active phase. **Reset communication defaults** restores the defaults above.

## Save evidence for support

**Activity log** retains the latest 64 events for the session. Each entry includes a local date and time. Serial operation entries include the request number and interface, with progress, results, backup paths, line settings, and any register errors. Connection changes and automatic polling failures are recorded; successful automatic polling is omitted from this visible list. **Copy log** reports whether the system clipboard write succeeded. **Save log…** exports the timestamped entries to a text file. The visible activity list resets on restart. Persistent process logs remain available through Export diagnostics.

**Export diagnostics…** creates one text file for support from persistent logs under `%LOCALAPPDATA%\Polygon\Device Configurator\Logs`. These include build identification, background operation summaries, errors, and Rust panic backtraces. Up to ten recent process logs are retained, each with a 5 MiB rotation threshold and one previous file. Clearing the visible activity list does not delete these files. Logs can contain device identifiers and local file paths; review the export before sharing. Native crashes, forced termination, and power loss do not guarantee a final log entry or a crash dump.

Include the build identifier from the window title, the local time of the failure, affected slave addresses and the selected point table when handing off a diagnostic export. Clear errors acknowledges displayed failures; it does not repair communication.
