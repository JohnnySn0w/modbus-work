# Runtime architecture

One Windows Rust process contains the egui interface and hardware service, connected by typed commands/events. The hardware service owns serial transactions; the UI owns navigation, selection, retained display data and explicit action scheduling. No Python/PowerShell runtime is required.

Modbus Bridge transport maintains a persistent handle and continuous receive worker. Bounded prompt handling and same-settings receive recovery avoid blind command retransmission. Device profiles own addresses/types/units; Modbus Bridge firmware 3.6 parsing/writing owns the eight-column TSV contract. Direct Modbus uses its own adapter backend and bus-ownership guard.

Configuration files use validated TSV and durable saves. Programming refreshes identity/table before writing, compares against the reviewed table, backs up, checks acknowledgements and verifies exported replacement. Uncertain writes pause normal activity for explicit recovery. There is no automatic rollback.

Last-good display retention is separate from fresh acquisition history. Failed samples never fabricate zero or reinsert retained values as fresh history. USB route identity and point definition distinguish history series. Display units do not change native data.

Native reference assets contain descriptions, maps, photos and manuals, not staged measurements. The generated Python baseline is extended with reviewed ATI/HMD65 additions. The UI still contains some per-model presentation logic; a completely data-driven device framework is an evolution goal, not an achieved claim.

IAQ console handling and firmware service integration are future modules. The [firmware design](FIRMWARE-UPDATES.md) requires exclusive handoff from normal polling to the external programmer and back. See [native build guide](../native/modbus-configurator/README.md).

## Source organization

The native entry point owns startup and shared state. Event handling, command scheduling, configuration files, preferences, and rendering are in app_events, app_commands, app_configuration, app_settings, and app_frame. Bundled reference extraction is in reference_files. Device pages and Modbus Bridge report parsing have dedicated submodules. See [code style](CODE-STYLE.md) for formatting and documentation expectations.
