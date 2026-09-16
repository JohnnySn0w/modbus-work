# Python / Rust parity

Rust is the supported Windows deliverable. Python remains a prototype/reference source; its staged readings never become Rust live data. Native reference generation preserves the Python register baseline and adds reviewed extensions in tools/native_reference_extensions.py.

| Workflow | Rust state |
|---|---|
| Dynamic USB discovery and Modbus Bridge identification | Implemented; observed routes are not fixed COM numbers |
| Persistent Modbus Bridge reads and receive recovery | Implemented; locally exercised |
| Modbus Bridge program, backup, restore and readback | Implemented and DPT146 bench-tested; power-loss persistence pending |
| File selection, saving and backup history | Implemented; Windows GUI acceptance recorded |
| DPT146 direct adapter | Implemented and bench-tested |
| HMD65 and WattNode adapter/profile support | Implemented; physical acceptance pending |
| ATI F12/PAA | Modbus Bridge candidate and reference decoding only; direct adapter pending |
| History/chart/CSV, settings, branding, manuals | Implemented |
| IAQ native identity/readings/console backup | Pending; historical Python observations do not prove Rust support |
| Radio configuration writes | Not qualified/enabled as a native commissioning workflow |
| Firmware package inspection and flashing | Pending in Rust; documented CLI approach, no flash execution |

Reference radio profiles are separate from regional display-unit preferences. Only the historically enabled US915 profile is available; regional firmware/hardware qualification is a separate requirement. Never infer radio capability from Windows region.

Detailed acceptance: [status](../CURRENT-STATUS.md), [goals](../GOALS.md), [coverage](RUST-COVERAGE.md). Original migration notes are preserved in docs/archive/2026-09-10-before-documentation-pass/.
