# HMD65 E5 bridge configuration

Both metric and non-metric E5 bridge tables use one-based manual register numbers. Metric measurements start at 1; non-metric measurements start at 129. Each table contains eight measurements and three status entries, for 11 entries total.

The error-code pair beginning at register 514 is excluded. Remaining status registers are 513, 518 and 519. Slave address defaults to 1 and can be changed in Configuration.

This is an E5 bridge configuration convention. Manufacturer register references and direct Modbus addressing remain zero-based. Floating-point word order remains HH pending verification.
