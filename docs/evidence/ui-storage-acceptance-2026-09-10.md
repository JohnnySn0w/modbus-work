> Historical evidence: observations and software state at the recorded test date. Earlier names, COM numbers, tool behavior and acceptance claims are preserved as evidence, not current instructions. See [current status](../../CURRENT-STATUS.md).

# UI and storage failure acceptance

The running Windows build was checked with live DPT146 readings. Rounded values, correct never-read disconnected wording, disabled unattached-device Read now, collapsed reference descriptions and exact-table Diagnostics label selection are implemented. Raw stored measurement values are unchanged. Device pages no longer show the global bridge timestamp.

Full Rust tests pass. The new integration test routes a real filesystem failure (backup root is an existing file) through BridgeSession::program, verifies UnsafeState and the no-changes message, observes no import or TSV writes, and confirms the original file survives. Existing tests cover simulated disk full, failed atomic saves, change/revert backup history, cancellation and uncertain writes. Clippy passes with warnings denied.

Automatic backup failures are reported separately from transport state: live reads continue and programming still requires a successful durable backup. A previous restricted Windows launch demonstrated visible backup errors while real readings continued; normal launch recovered backup access.

Previously completed physical programming/restoration evidence is in native-rust-program-restore-2026-09-10.md. Full power-loss persistence is still pending user removal of both external power and USB; a host USB reset is not equivalent.
