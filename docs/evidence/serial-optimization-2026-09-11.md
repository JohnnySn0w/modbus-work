# Modbus Bridge serial timing baseline

Read-only Windows bench test on 2026-09-11 using the production Rust transport
and `capture_read` harness. Windows enumerated one Synetica USB console on COM5
(the observed assignment, not a required port). No adapter or running
configurator process was detected. The session verified ENL-MOD-32 firmware 3.6
and exported its existing table before reading. No configuration was changed.

## Results

| Test | Cycles | Read duration | Result |
| --- | --- | --- | --- |
| Persistent session, five-second idle gaps | 10 | 2698–2810 ms normally; 4837 ms with table refresh | Eight values per cycle, zero exceptions |
| Fresh session, no idle gaps | 10 | 2772–2811 ms | Eight values per cycle, zero exceptions |

No measurement cycle needed receive recovery. Initial console access needed
one recovery call in the first session. After normal close, the second session
needed seven recovery calls and one bounded console wake, then completed its
backup and all reads without a physical replug. Both processes exited normally.

Recovery here means reapplying the existing USB baud setting on the open handle;
it is not a device reset or driver restart. The two sessions used identical
production transport settings. Only the harness idle interval differed.

The evidence supports retaining persistent sessions. Shorter idle gaps increased
the sampling frequency without improving individual request latency. Startup
recovery is a better candidate for further controlled optimization than reducing
the measurement timeout. No production timing changes were made on this basis.

This is a short test of the currently attached Modbus Bridge and configured sensor,
not an overnight soak, independent sensor identity verification, adapter test,
or firmware-transfer qualification. Earlier September 10 timings included four
device exceptions and are not a controlled performance comparison.

## Reproduction

Build `capture_read` with the Windows MSVC toolchain. Stop other applications
using the selected console, enumerate its current port, then run sequentially:

```text
capture_read <port> <new-private-trace-path> 10 5000
capture_read <port> <another-new-private-trace-path> 10 0
```

The harness now reports recovery counts per measurement and accepts an optional
idle interval of 0–60000 ms (default 5000). Raw traces can contain credentials;
this run's traces and backups remain in ignored `tmp/serial-optimization/`.
Only aggregate results are recorded here. Formatting and example Clippy checks
passed, and the updated harness was built and exercised on hardware.
