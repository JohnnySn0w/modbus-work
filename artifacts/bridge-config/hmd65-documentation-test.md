# HMD65 Modbus Bridge configuration

Both metric and non-metric Modbus Bridge tables use one-based manual register numbers. Metric measurements start at 1; non-metric measurements start at 129. Each table contains eight measurements and one device-status entry, for nine entries total.

Registers 514–515, 518 and 519 are excluded. Device status at register 513 remains included. Slave address defaults to 1 and can be changed in Configuration.

This is a Modbus Bridge configuration convention. Manufacturer register references and direct Modbus addressing remain zero-based. Floating-point word order remains HH pending verification.
