# Rust coverage

Measured after formatting and module refactoring, 2026-09-10: **151 tests pass; 92.40% combined line coverage**. All-target Clippy passes with warnings denied. The earlier test-only baseline was 145 tests / 90.22%; the new combined report includes six additional tests and actual offline Windows GUI startup and capture of 23 views. The fresh headless result is 91.08%; desktop startup/rendering raises combined coverage to 92.40%. These figures have different execution scope.

Run `./tools/Check-Rust.ps1` for headless tests (90% floor), or `./tools/Check-Rust.ps1 -IncludeGui` on a Windows desktop to add startup/rendering coverage (92% combined floor). GUI mode also checks the headless floor before merging. Each run cleans old coverage artifacts first, preventing stale profiles from contributing. The GUI uses the offline backend and isolated application data; no physical serial interfaces are opened.

| Source | Line coverage |
|---|---:|
| adapter.rs | 87.17% |
| app_commands.rs | 94.41% |
| app_configuration.rs | 93.13% |
| app_events.rs | 88.93% |
| app_frame.rs | 95.45% |
| app_settings.rs | 92.13% |
| brand.rs | 100.00% |
| bridge.rs | 87.09% |
| parsing.rs | 93.75% |
| capture_views.rs | 100.00% |
| catalog.rs | 98.41% |
| catalog_view.rs | 92.05% |
| config_file.rs | 94.26% |
| console.rs | 100.00% |
| device_art.rs | 99.49% |
| history.rs | 99.00% |
| history_view.rs | 93.10% |
| last_good.rs | 97.75% |
| main.rs | 90.91% |
| reference.rs | 98.10% |
| reference_files.rs | 88.46% |
| service.rs | 84.01% |
| technician_view.rs | 95.86% |
| device_pages.rs | 93.79% |
| transport.rs | 75.11% |
| troubleshooting.rs | 100.00% |
| units.rs | 97.65% |

All application source modules remain in scope. Separate test/example files and dependencies are excluded; inline tests can contribute to LLVM totals. This is not production-only, branch or MC/DC coverage. No new source exclusions were added.

Test coverage includes focused cases for USB metadata normalization across arbitrary COM routes, missing USB identity, invalid/oversized configuration files, failed destinations, incomplete/duplicate profiles, fragmented terminal control strings, and the offline hardware lockout. USB descriptor mapping was extracted from OS enumeration without changing its behavior.

## Remaining work toward full coverage

- Service and transport: unexercised coordination/race/error branches and native serial driver wrappers. Add deterministic fault injection; do not open bench hardware merely to cover a line.
- GUI: native dialogs, reference launching and remaining state/error branches.
- Catalog, parser and storage: remaining malformed-input and OS failure paths.
- Startup: normal hardware-enabled startup and previous-installation migration are not covered by the offline desktop capture.

100% remains a goal, not a result. Even 100% line coverage would not establish hardware reliability or exercise every branch.

Fresh evidence: artifacts/review/refactor-final-check.log (local) and docs/evidence/rust-refactor-2026-09-10.json. Earlier evidence remains historical. Formatting and module extraction change LLVM line accounting; this small percentage change is not an additional behavioral test. Reports are under native/modbus-configurator/target/coverage (coverage.json, lcov.info, html/index.html); GUI runs also save headless-coverage.json before merging. Requires MSVC Rust, llvm-tools-preview and cargo-llvm-cov.
