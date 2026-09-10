> Historical evidence: observations and software state at the recorded test date. Earlier names, COM numbers, tool behavior and acceptance claims are preserved as evidence, not current instructions. See [current status](../../CURRENT-STATUS.md).

# History, restart recovery and device support — 2026-09-10

A normal Alt+F4 close of the old GUI and launch of the new Windows executable
returned to live DPT146 readings without touching USB. The new run reported its
first captured reading at 13:45:02 local and continued advancing. Its fresh
export matched the eight-point DPT146 preset. Screenshot evidence is in
`artifacts/archive/review/gui-history-recovery/`; the configuration screenshot shows exact match.
This is one observed restart, not overnight/physical hotplug qualification.

The History page plotted actual successive temperatures. A native Save As dialog
exported `artifacts/archive/review/gui-history-recovery/live-history.csv`: 104 point samples,
13 eight-point cycles. CSV was independently read back. Zero status values remain
zero. Failed point values are blank with a status; retained UI readings are merged
only after history ingestion. Read errors and observed route loss insert gaps.
Charts also break after 30 seconds between samples. History is bounded at 50,000
point records and currently lasts only for the running application session.

Regression tests cover USB replacement between commands without an Inventory
request (no new serial write; previous handle dropped), restart from an observed
completed-read prompt, real zero versus missing sample, export-only exclusion,
COM reassignment identity, and changed point definitions. All 80 tests and
all-target warnings-denied Clippy passed before the final presentation-only edits.

Device references distinguish hardware-tested bridge/DPT146 paths, implemented
but unqualified HMD65/WattNode/adapter paths, and reference-only IAQ. Topology and
detail connectivity now recognize matching HMD65 and WattNode bridge presets.
DPT146 health mirrors Python's Fault=1, Online=1, Error=0 rule using the mapped
live registers; missing, failed or stale status does not claim Online.

Full power-loss verification remains pending. The operator was asked to disconnect
both external power and USB and reconnect after the lights extinguish. No response
confirming the physical cycle has been received. No programming writes were
performed during this batch. IAQ console acquisition/backup and firmware-package
inspection remain unfinished parity work.

Final installed release was restarted normally a second time and automatically
returned to DPT146 values (13:51:33 local observed). Updated captures are in
`artifacts/archive/review/gui-history-final/` (18 view/profile images). No physical USB replug
was requested for either app restart; the separately requested full power-cycle
remains unconfirmed. Final build and all-target Clippy passed.
