# Native Rust application design

Last updated: 2026-09-09

## Decision

Replace the prototype with one native executable named `modbus-configurator.exe`. The GUI and hardware services are both implemented in Rust and packaged as a single binary that can be handed to a technician or reviewer without Python, PowerShell, or a separate runtime.

This is more than a language translation. The reliability improvement comes from one persistent process, one owner per COM port, and explicit protocol state machines instead of short PowerShell processes that sleep for fixed intervals and search for one exact prompt. The UI and hardware layers remain separate Rust modules even though they ship in one executable.

## Runtime topology

```text
modbus-configurator.exe
    |-- eframe/egui technician UI
    |       `-- typed commands and events
    `-- hardware service
            |-- port inventory and USB metadata
            |-- ENL-MOD-32 console state machine
            |-- native enLink console state machine
            |-- direct Modbus RTU master
            `-- transcript, backup, and verification services
                    |
                    v
               Windows COM ports
```

Device profiles and bridge tables remain data artifacts. They are not compiled into GUI screens and they do not own serial-port behavior.

## Responsibilities

The GUI owns:

- technician navigation, cards, tables, help, warnings, and confirmations;
- selection of a reviewed device profile or bridge configuration;
- display of progress, decoded readings, diffs, and results;
- operator-selected export locations.

The hardware agent owns:

- enumerating serial interfaces and reporting USB VID, PID, serial number, and current availability;
- maintaining a single exclusive actor and operation queue for each COM port;
- passive banner collection and deterministic product fingerprinting;
- authenticated console navigation only for explicit technician actions;
- direct Modbus RTU reads, safe fingerprint probes, and decoded result transport;
- bridge backup, import, exported readback, comparison, and validation;
- native enLink configuration reads and later field-level writes;
- monotonic deadlines, bounded retries, cancellation, recovery, and structured error codes;
- raw transcripts in an expert log while keeping routine UI messages concise.

COM numbers are routes, never product identities. Product identification must combine USB metadata with a protocol response, console banner, documented identity register, or a sufficiently specific read-only register fingerprint.

## Internal command contract

Use strongly typed Rust commands and events across channels between the UI thread and hardware-service thread. Serialize the same types as newline-delimited JSON for replay fixtures, diagnostics, and automated tests. No child process or named pipe is required in the shipped application.

Every command includes:

- `request_id` - unique identifier supplied by the GUI;
- `operation` - stable command name;
- `port` - requested route when an operation needs one;
- `expected_identity` - optional model and firmware compatibility gate;
- `payload` - operation-specific data.

The agent emits:

- `port_snapshot` - current interfaces, identities, busy state, and passive observations;
- `progress` - durable operation stage suitable for the detail view;
- `prompt_state` - expert diagnostic state, not raw menu text as application logic;
- `result` - structured values, backup metadata, or verification evidence;
- `error` - stable code, plain-language message, recoverability, and optional transcript reference.

Raw serial text and PowerShell exception text must not be used as a UI contract.

## Serial and console state machines

Each console adapter recognizes states from an accumulated byte stream. The parser normalizes CR/LF variants, NUL bytes, ANSI control sequences, echoed input, and fragmented reads. It waits for recognized state or a monotonic deadline, not a fixed sleep followed by a single exact string comparison.

The initial ENL-MOD-32 state model is:

```text
Closed -> Banner -> Password -> Main menu -> Modbus menu -> Import/export
                                     |             |
                                     `-> Radio     `-> Read/test
```

Before a write, the adapter must:

1. drain input until a bounded quiet window;
2. identify the current state from the rolling transcript;
3. back out of a known submenu with bounded `X` transitions when necessary;
4. re-authenticate when the device is at its password prompt;
5. verify exact bridge model and firmware;
6. export and validate a native backup;
7. apply only the requested native fields;
8. export again, compare the readback, and run the device's read/test command;
9. return success only when all required verification stages pass.

If the state remains unknown, the adapter may send a harmless line ending and attempt one bounded resynchronization. It must stop without writing if identity, state, or backup cannot be established.

## Modbus-bus safety

- Only one process may own a serial port.
- The agent must serialize scanning and explicit actions on that port.
- Active Modbus fingerprinting is allowed only when this tool is known to be the intended master.
- If another master may be present, show the adapter as unavailable or ambiguous and do not transmit probes.
- Configuration writes remain disabled unless the profile classifies the field as writable and the workflow has backup, preview, confirmation, readback, and recovery evidence.

## Rust implementation outline

Start with a Cargo workspace under `native/modbus-configurator/`. Prefer a blocking serial reader per active port and message passing to the coordinator; asynchronous Rust is not required for the first version.

Expected building blocks:

- `eframe` and `egui` for the native cross-platform GUI without a browser or webview runtime;
- `serialport` for Windows serial access;
- `serde` and `serde_json` for the command/event and replay schema;
- `thiserror` for stable internal error categories;
- `tracing` and `tracing-subscriber` for structured diagnostic logs.

Keep protocol parsers independent of the real serial transport. A replay transport must be able to feed recorded transcripts in arbitrary chunk sizes and timings.

## Migration plan

1. Scaffold the eframe/egui application, typed command/event model, and replayable JSON fixture format.
2. Reproduce the current connected-device diagram, detail pages, register tables, help, and backup/configuration controls in Rust.
3. Implement port inventory and passive Synetica banner discovery.
4. Implement ENL-MOD-32 login, menu recovery, read-only export, and Read All using captured transcript fixtures.
5. Validate those operations on the physical bridge without writes.
6. Add native backup, guarded import, exported readback comparison, and recovery artifacts.
7. Move direct USB-COMi-TB Modbus polling and device fingerprinting into the hardware service.
8. Move IAQ console reads and credential workflows into the hardware service.
9. Retire the Python/Tk and PowerShell runtime paths after feature parity and hardware validation.
10. Build a release-mode single executable and verify it on a clean Windows machine.

## Verification plan

Automated fixtures must cover fragmented prompts, delayed output, stale submenus, repeated banners, CR/LF variations, unexpected echo, disconnect/reconnect, cancellation, and firmware prompt variations. Hardware acceptance must separately prove:

- discovery while devices are plugged and unplugged;
- no simultaneous scanner/action ownership;
- recovery from every known console screen;
- read-only bridge export and live DPT146 reads;
- backup fidelity using only native exported fields;
- guarded import, exact readback, Read All success, and reboot persistence;
- clear user-visible errors with transcript detail available separately.

## Packaging direction

The first technician delivery is a release build of `modbus-configurator.exe`. Reviewed device profiles, help text, icons, and default configurations are compiled into the executable; operator backups and commissioning reports remain ordinary external files. The Windows release should use a static C runtime where the dependency set permits it and must be smoke-tested on a clean machine. Signing, an optional installer, automatic log collection, and firmware-update integration follow bench validation. Firmware flashing remains a separate, explicitly authorized workflow.
