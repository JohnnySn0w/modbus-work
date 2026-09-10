> Historical evidence: observations and software state at the recorded test date. Earlier names, COM numbers, tool behavior and acceptance claims are preserved as evidence, not current instructions. See [current status](../../CURRENT-STATUS.md).

# Persistent bridge session verification

Windows COM5, ENL-MOD-32 firmware 3.6, DPT146, 2026-09-10.

The production transport opens one serial handle, owned by a continuous I/O
worker. Routine writes do not close/reopen it. The worker drains input during
GUI idle periods and backup writes. A pending reply that stalls for 500 ms may
reapply the existing 115200 USB baud setting using SetCommState on that same
handle. This is USB receive recovery; it does not reset the device or change
the downstream Modbus baud setting. Replies are preserved, commands are not
resent, and the entire operation remains deadline-bounded.

Earlier controlled experiments: persistent reading alone, ClearCommError and
FlushFileBuffers did not recover command replies. DTR low/high also did not.
Reapplying the unchanged serial baud setting delivered a pending 525-byte
password/banner reply on the existing handle. No speculative configuration
commands or physical USB reconnect were used.

## Live acceptance

- First run: native backup followed by 20 consecutive verified Read All cycles
  on one handle, with five seconds idle between cycles. The private trace
  contains exactly 20 measurement-completion blocks. Every cycle passed point
  identity/completeness and 8/4 summary checks.
- Cached configuration was refreshed during the run (including cycle 8), through
  the same session. Routine reads took approximately 4.1 seconds; cycle 8 took
  6.23 seconds including the table refresh.
- Second run: closed the first session normally, opened a fresh session, saved
  another backup, and completed five more verified cycles without a physical
  replug. Summary follows.

```text
Read 1: 8 values, 4 exceptions, 4080 ms; temperature Some(23.786575)
Read 2: 8 values, 4 exceptions, 4128 ms; temperature Some(23.787643)
Read 3: 8 values, 4 exceptions, 4120 ms; temperature Some(23.797531)
Read 4: 8 values, 4 exceptions, 4121 ms; temperature Some(23.790665)
Read 5: 8 values, 4 exceptions, 4125 ms; temperature Some(23.800156)
```

Every cycle returned eight real DPT146 values and four explicit Illegal Data
Address exceptions from the extra configured points. Those exceptions remain
configuration/device results, not transport failures. No point table was changed.

This is a several-minute soak plus a fresh-session check, not an overnight or
all-device reliability claim. Private raw traces remain under ignored target/.
The public evidence contains only measurement summaries.

## GUI acceptance

The rebuilt release was launched after the second bench session closed. Its
overview showed fresh DPT146 values at 08:48:22 local. Opening the DPT146 detail
page showed the next automatic reading at 08:48:32 local: temperature 23.820908 C,
dew/frost point 9.360622 C, atmospheric dew point 9.373842 C and moisture
11753.492188 ppmv. Per-value timestamps advanced and the page remained identified.
The release was left running on that page. All 65 Rust tests and Clippy passed.
