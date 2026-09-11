# Windows CI and beta releases

The workflow is `.github/workflows/windows.yml`. It becomes active after that file and the native source are pushed to GitHub. Creating it locally does not start a hosted run.

## Checks and build

Pull requests into main, pushes to main, and manual runs check rustfmt, all-target Clippy, the test suite, and the 90% headless line-coverage floor. Rust 1.89.0 and cargo-llvm-cov 0.9.1 are pinned. Dependencies are fetched from Cargo.lock before the existing offline scripts run.

The Windows 2022 runner builds the x64 executable with the static MSVC runtime and checks runtime imports. It verifies the ZIP checksum, executable checksum, README, and dependency notices. The package metadata records the source commit. Executable, ZIP, and checksums are retained as Actions artifacts for 14 days, alongside coverage reports.

Hosted CI does not launch the native window or touch physical serial hardware. Continue using `tools/Check-Rust.ps1 -IncludeGui` and the normal `tools/Test-WindowsPackage.ps1 -ZipPath ...` locally for desktop rendering checks. `-SkipLaunch` is explicitly an integrity-only package check.

## Publish a beta

After the workflow and application source are upstream, create and push a new tag such as `v0.1.0-beta.1` on the intended commit. The accepted form is `vMAJOR.MINOR.PATCH-beta.N`.

The tag runs the same checks and build. Only after success does the release job upload the EXE, ZIP, and both SHA-256 files. It creates a draft first and publishes it as a prerelease after upload succeeds. An interrupted draft upload can be retried. An already-published beta is never overwritten; use a new tag for changes.

The ZIP is the preferred download because it includes the README and dependency notices. Beta publication does not mark the release as the latest stable version. Manual workflow runs produce downloadable artifacts but do not publish a release.

## Credentials and boundaries

The workflow uses GitHub's built-in token. The build has read-only repository access; only the tag-release job has contents-write permission. Checkout credentials are not persisted. Actions are pinned to immutable revisions. Pull requests cannot run the release job.

No personal access token, local login, signing key, or private Synetica procedure is required. Binaries are unsigned. Publishing is still subject to repository Actions policies and permissions. A local approval block on pushing source is separate from the workflow's release token and must be resolved before activation.

References: [GitHub workflow syntax](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax), [GitHub CLI release creation](https://cli.github.com/manual/gh_release_create).

## Local verification

Workflow checked with actionlint 1.7.7; embedded PowerShell blocks passed parser checks. A versioned package was built locally and passed integrity checks. Invalid version strings and incorrect checksums were rejected. A hosted Actions run and release upload have not yet been executed.
