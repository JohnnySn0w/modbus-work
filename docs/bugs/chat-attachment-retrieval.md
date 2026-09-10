> Historical evidence: observations and software state at the recorded test date. Earlier names, COM numbers, tool behavior and acceptance claims are preserved as evidence, not current instructions. See [current status](../../CURRENT-STATUS.md).

# Recent ChatGPT attachments omitted by Codex read_thread

Status: prepared locally; not submitted.
Observed: September 10, 2026, Codex desktop on Windows.

## Problem

Codex can retrieve up-to-date messages from an existing ChatGPT conversation, but its read_thread attachment list repeatedly exposes only three older files. Two newer uploads (a branding-guidelines PDF and fonts.zip) are omitted. This prevents use of the uploaded files across chats despite the latest messages being visible.

## Reproduction

1. Use an existing ChatGPT conversation with older uploaded files.
2. Upload a new PDF, then a font ZIP in later messages.
3. From a Codex task, call mcp__codex_app__read_thread with that conversation ID, includeOutputs=true and turnLimit=1 or 2.
4. Inspect both turns and attachments in the response.
5. Repeat the call after the latest upload has completed.

## Expected

The attachments list includes retrievable new uploads, or explicitly explains why each attachment cannot be fetched.

## Actual

The latest user message is present, with '[User attached 1 file; file contents were not included]'. The latest assistant response also refers to the font family. The attachments array still contains exactly the same three older files and no fonts.zip or branding PDF. Repeated calls produce fresh local temporary paths for the older files, while omitting both new uploads. No error is returned.

Moving the branding PDF to Library was also reported by the user, but no direct Library retrieval tool was available; this is a separate access limitation, not proof that the same bug affects Library.

## Impact

Cross-chat text retrieval succeeds, while recent file retrieval silently fails. Users must reattach files or manually copy them into the local workspace.

## Scope and uncertainty

Observed repeatedly on one conversation. No claim of a confirmed root cause, broad outage, or specific file-size/type limit. No file contents, credentials, or private conversation text beyond the minimal attachment marker are included in this report.
