# Windows CI and beta releases

The workflow is `.github/workflows/windows.yml`. It becomes active after that file and the native source are pushed to GitHub. Creating it locally does not start a hosted run.

## Checks and build

Pull requests into main, pushes to main, and manual runs check rustfmt, all-target Clippy, the test suite, and the 90% headless line-coverage floor. Rust 1.89.0 and cargo-llvm-cov 0.9.1 are pinned. Dependencies are fetched from Cargo.lock before the existing offline scripts run.

The Windows 2022 runner builds the x64 executable with the static MSVC runtime and checks runtime imports. It verifies the ZIP checksum, executable checksum, README, and dependency notices. The package metadata records the source commit. Executable, ZIP, and checksums are retained as Actions artifacts for 14 days, alongside coverage reports.

Hosted CI does not launch the native window or touch physical serial hardware. Continue using `tools/Check-Rust.ps1 -IncludeGui` and the normal `tools/Test-WindowsPackage.ps1 -ZipPath ...` locally for desktop rendering checks. `-SkipLaunch` is explicitly an integrity-only package check.

## Parallel jobs and caches

Formatting/Clippy, tests/coverage, and Windows packaging run on three independent runners. Cargo uses each runner's available CPUs for compilation, and the Rust test harness retains its normal parallel execution. Tests run once through the coverage job. Beta publication depends on all three jobs succeeding; a downloadable build artifact alone does not mean the other checks passed.

The local setup action shares Cargo registry and Git dependency downloads through a lockfile-keyed cache. Compiled outputs have separate caches for lint, coverage instrumentation, and static-runtime release builds. Those keys include OS, architecture, pinned Rust toolchain, job kind, lockfile hash, and commit; compatible prior commits can restore dependency outputs. Cargo still runs to validate fingerprints and rebuild changed inputs. Incremental compilation is disabled to limit cache size.

The cargo-llvm-cov installation is cached by platform, toolchain, and version, and installed only on a cache miss. Coverage profiles and reports are not cached; the coverage script clears prior profiles before running tests. Cache misses still perform the complete install, fetch, and build. Bump the `v1` cache namespace when changing compiler flags or cache layout. Cache hits and timing improvements require verification on hosted runs.

## Publish a beta

After the workflow and application source are upstream, create and push a new tag such as `v0.1.0-beta.1` on the intended commit. The accepted form is `vMAJOR.MINOR.PATCH-beta.N`.

The tag runs the same checks and build. Only after success does the release job upload the EXE, ZIP, and both SHA-256 files. It creates a draft first and publishes it as a prerelease after upload succeeds. An interrupted draft upload can be retried. An already-published beta is never overwritten; use a new tag for changes.

The ZIP is the preferred download because it includes the README and dependency notices. Beta publication does not mark the release as the latest stable version. Manual workflow runs produce downloadable artifacts but do not publish a release.

## Credentials and boundaries

The workflow uses GitHub's built-in token. The build has read-only repository access; only the tag-release job has contents-write permission. Checkout credentials are not persisted. Actions are pinned to immutable revisions. Pull requests cannot run the release job.

No personal access token, local login, signing key, or private Synetica procedure is required. Binaries are unsigned. Publishing is still subject to repository Actions policies and permissions. Source pushes use the repository's normal permissions; release authentication is provided inside Actions.

References: [GitHub workflow syntax](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax), [GitHub CLI release creation](https://cli.github.com/manual/gh_release_create).

## Local verification

Workflow checked with actionlint 1.7.7; embedded PowerShell blocks passed parser checks. A versioned package was built locally and passed integrity checks. Invalid version strings and incorrect checksums were rejected. A hosted Actions run and release upload have not yet been executed.
