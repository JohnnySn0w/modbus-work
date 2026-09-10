> Historical evidence: observations and software state at the recorded test date. Earlier names, COM numbers, tool behavior and acceptance claims are preserved as evidence, not current instructions. See [current status](../../CURRENT-STATUS.md).

# Native bridge programming and restore acceptance

Bench date: 2026-09-10. User approved applying the eight-point DPT146 table,
removing points 9–12, and testing restore. ENL-MOD-32 firmware 3.6 on COM5;
DPT146 remains connected at slave 1, 19200 baud, 8N2.

## Observed defect and repair

The first GUI programming attempt saved its mandatory backup and completed the
delete batch. It then timed out, stopped, and paused automatic polling. A console
check showed zero points. No blind row retry or automatic rollback was performed.
The existing automatic backup contained all original 12 rows.

A traced recovery import acknowledged all eight DPT146 rows. Actual firmware ends
with `Import Finished. Results:`, the resulting table, and `Press a key to continue:`.
The code incorrectly required a line ending at `Import Finished`. The fix recognizes
the complete hardware header and requires the final Continue prompt before sending
the next command. A sanitized hardware fixture now tests both the full response and
truncation before the Continue prompt. Private raw traces remain in ignored target/.

## Successful acceptance

The developer harness uses the same production BridgeSession::program implementation
as the GUI, including fresh review comparison, mandatory synced backup, per-row
acknowledgements and exact export verification. It retained one handle throughout
each program/read sequence.

1. Restore: current eight-point table was backed up, then replaced with the original
   12-point backup. The final exported table matched exactly. Three consecutive reads
   returned eight values and four expected Illegal Data Address exceptions (9–12).
2. DPT146: the 12-point table was backed up and replaced with the eight-point profile.
   The final exported table matched exactly. Three consecutive reads returned eight
   values, zero exceptions, and a matching 8/0 summary.
3. Final state: the bridge was left on the eight-point DPT146 configuration. The
   corrected release executable was installed and the GUI relaunched. At 09:56:55
   local it displayed fresh values: temperature 24.57 C, dew point 10.15 C and
   atmospheric dew point 10.14 C. This confirms reads after session close/reopen.

Artifacts: [restored table](bridge-restored-verified-2026-09-10.tsv),
[DPT146 table](bridge-dpt146-verified-2026-09-10.tsv), and
[six actual read results](bridge-program-restore-readings-2026-09-10.json).

Power-cycle persistence has not been tested. A clean close/reopen and GUI restart
are not equivalent to removing bridge power. Full file-dialog/backup-history UI
acceptance and an overnight soak remain separate goals.
