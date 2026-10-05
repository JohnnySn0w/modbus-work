# WattNode WND-M1-MB integer bridge profile

The default table has 17 entries: non-resettable net energy; AN, BN, CN, AB, BC and CA voltages; three integer currents; firmware; power-failure count; lifetime operating seconds; three configured current-transformer ratings; and the current scale required to interpret the integer currents.

All bridge multipliers are 1. Energy is in 0.1 kWh and voltage in 0.1 V. Current is raw counts: amperes = value × the matching CtAmps1/2/3 (1604–1606) ÷ CurrentIntScale (1622). Never assume a fixed amps-per-count multiplier. Missing or zero current scale prevents conversion.

Net energy is signed 32-bit, low-word-first. Lifetime operating time is unsigned 32-bit, low-word-first. [CONFIG] identifies writable meter settings. This bridge profile only reads those settings; it does not write them. Hardware verification of the revised selection is pending. Existing backups retain their original register selections.

Source: bundled WND-M1-MB-Ref-1.10 manual, sections 3.4.6–3.4.9, 3.5 and 3.6. Register 1216 is C-to-neutral; 1218–1220 are AB/BC/CA.
