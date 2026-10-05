# Configuration controls audit — current resolution

Reviewed 2026-09-29. The original pre-implementation audit is archived; its missing-programming findings are no longer current.

Configuration now provides one device-list editor for profiles, imported files and backups, a verified Modbus Bridge target, changes preview, **Program Modbus Bridge**, **Back up Modbus Bridge**, **Load backup**, **Save configuration file** and **Copy configuration table**. An orange unavailable-target banner distinguishes disabled hardware actions from available offline selection/saving. Manual work queues behind background reads.

Program/restore is implemented and locally validated with DPT146. Required fresh review, backup, acknowledgements and exported verification remain enforced. Physical full-power-loss persistence is pending. Point-table success is separate from downstream sensor read success or physical identity.

Reference file actions now open named offline PDF manuals; configuration TSVs are handled in Configuration. Routine successful reference-open messages were removed. Device help navigates to Troubleshooting.
