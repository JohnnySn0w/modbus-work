# Code style

Prefer precise names and short explanations. Readability takes priority over line counts.

## Formatting and checks

Rust uses rustfmt with the crate's rustfmt.toml. Run `cargo fmt --manifest-path native/modbus-configurator/Cargo.toml --all` to format it. `tools/Check-Rust.ps1` checks formatting, runs all-target Clippy with warnings denied, and runs the tests and coverage gate. Add `-IncludeGui` for offline Windows startup and rendering coverage.

## Size and responsibilities

- Review modules as they approach 500–800 lines. Split by responsibility, such as parsing, persistence, or a specific view. Small modules are fine when the boundary is useful.
- Aim for functions that can be understood in one screen, usually around 60 lines. Longer UI builders or protocol state machines need a clear purpose and readable sections; do not create helpers solely to reduce a count.
- Prefer a named input structure over a long positional argument list. Keep state ownership and side effects explicit.
- Treat these as review targets, not mechanical limits. Existing long functions remain candidates for gradual refactoring with behavioral tests.

## Documentation and wording

- Write clear technical English in interface text and operator instructions. Spell out abbreviations in prose and controls; preserve exact product names, terminal labels, file extensions, protocol tokens and standard measurement symbols where changing them would obscure the technical meaning.
- Use **TBD** for instructional content that has not been established. Do not invent troubleshooting sequences, default addresses, wiring instructions or expected readings to fill a gap.
- Instructions require documented technical support. E5 bridge and Vaisala DPT146 guidance can draw on recorded test results. Other device instructions require a specific manufacturer source; label them as documentation-based suggestions until tested. Application instructions must agree with implemented behavior.
- Keep research proposals and historical test records separate from operator instructions. A successful register read does not prove the physical sensor model. A saved profile describes how values are interpreted, not an independently verified identity.
- Start source modules with `//!` explaining their responsibility and important boundaries.
- Use `///` on functions to state the contract: what they do, relevant side effects, failure behavior, and any non-obvious preconditions. Public APIs and hardware/storage operations especially need this.
- A one-line comment is usually enough. Do not repeat the signature in prose or document obvious syntax. Tests should state the behavior they protect; avoid boilerplate doc comments that repeat test names.
- Say E5 bridge in user-facing text. Use concrete actions and plain status messages. Distinguish queued, running, stale, failed, and completed states.
- Explain why a constraint exists when it affects a decision. Keep developer details out of routine user flows.
- Make each description understandable without development history. Name the equipment, action or limitation directly. Prefer "USB adapter connection" to "transport", "serial port" to "route" when that is the intended meaning, and "hardware testing pending" to "candidate" or "unqualified". Do not imply sensor identification from a configuration match.

## Current structure

The binary entry point owns shared state and startup. The app_events, app_commands, app_configuration, app_settings, and app_frame modules separate event handling, request ownership, configuration workflows, preferences, and rendering. Device detail/register views live under technician_view; console parsing lives under bridge. These modules preserve the existing behavior and tests.

Formatting is enforced now. Documentation and function-size cleanup are progressive review requirements, not a claim that every existing function already meets them.
