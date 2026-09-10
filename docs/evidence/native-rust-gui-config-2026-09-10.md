> Historical evidence: observations and software state at the recorded test date. Earlier names, COM numbers, tool behavior and acceptance claims are preserved as evidence, not current instructions. See [current status](../../CURRENT-STATUS.md).

# Windows configuration workflow acceptance

2026-09-10, actual Windows GUI and connected DPT146 bridge.

- Saved the selected eight-point table through the native Save As dialog to an
  ignored acceptance TSV. Its SHA-256 matched the verified DPT146 export:
  `2D2B23DAA6961C3D17F238B94369CFDF54B6322ACA8F9F2AE69617E04460B147`.
- Opened the bridge-specific Backups menu. It listed the newer eight-point table
  and the original 12-point table. Selecting the original backup showed 12 points
  and four additions in the review, without programming hardware.
- Loaded the saved eight-point TSV through the native Open dialog. The preview
  returned to eight points and zero differences against the bridge.
- Clicked Program bridge on the loaded TSV in the corrected production GUI.
  It completed and displayed programming/readback success. Automatic readings
  resumed; the displayed last-reading timestamp advanced to 10:05:26 local.
- Saved again to the same path, accepted Windows' replace confirmation, and
  observed `TSV file saved.` The destination modification time advanced to
  10:07:07 local. This used the application's atomic save implementation.

The checks exposed a feedback bug: successful automatic Backup events cleared
unrelated save/load messages. Backup errors now have separate state; successful
background backups cannot erase file-action feedback. Messages can be dismissed.
The unused alternate backup-write path was removed. Backup menu entries now show
latest/earlier version and point count; their full paths remain available on hover.
Programming success no longer leaves a stale “fetching next” message after reads.

UI tests and warnings-denied all-targets Clippy pass. The updated executable was
installed at the normal release path. Storage-failure GUI acceptance and physical
power-cycle testing remain pending.

Earlier project notes and an actual captured menu show `R - Reboot`, but no reboot
was issued in this GUI acceptance pass. Software reboot and physical power-cycle
persistence must be recorded as separate acceptance checks.


After installing the feedback fix, the restarted GUI remembered the eight-point
selection and again listed the latest eight-point and earlier 12-point backups.
Selecting the latest backup loaded eight points with zero differences. Its load
confirmation remained visible as readings advanced from 10:09:28 to 10:10:17 local.
This verifies persisted backup history and selection after restart. A duplicate
point-count label noticed during inspection was removed; failed backup loads now
retain the previous source label as well as the previous selected table.
