from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class RegisterDefinition:
    name: str
    logical: str
    pdu: str
    data_type: str
    access: str
    unit: str
    description: str
    readout_label: str | None = None


def _r(name: str, logical: int | str, pdu: int | str, data_type: str,
       unit: str, description: str, readout: str | None = None,
       access: str = "R") -> RegisterDefinition:
    return RegisterDefinition(name, str(logical), str(pdu), data_type, access,
                              unit, description, readout)


DPT146_REGISTERS = (
    _r("Temperature", "5–6", "4–5", "F32 HL", "°C", "Measured gas temperature.", "Temperature"),
    _r("Dew/frost point", "7–8", "6–7", "F32 HL", "°C", "Dew point or frost point temperature.", "Dew / frost point"),
    _r("Atmospheric dew point", "11–12", "10–11", "F32 HL", "°C", "Dew point converted to atmospheric pressure.", "Atmospheric dew point"),
    _r("Moisture", "21–22", "20–21", "F32 HL", "ppmv", "Water-vapor volume concentration.", "Moisture"),
    _r("Absolute pressure", "45–46", "44–45", "F32 HL", "bara", "Absolute process pressure.", "Absolute pressure"),
    _r("Fault status", "513", "512", "U16", "", "Device fault indicator; preserve the raw value for diagnosis.", "Fault status"),
    _r("Online status", "514", "513", "U16", "", "Device online/measurement availability indicator.", "Online status"),
    _r("Error code", "516–517", "515–516", "U32 HL", "", "Current device error code; zero is expected when healthy.", "Error code"),
)


HMD65_REGISTERS = (
    _r("Relative humidity", "1–2", "0–1", "F32", "%RH", "Relative humidity measurement.", "Relative humidity"),
    _r("Temperature", "3–4", "2–3", "F32", "°C", "Air temperature measurement.", "Temperature"),
    _r("Dew point", "5–6", "4–5", "F32", "°C", "Calculated dew-point temperature.", "Dew point"),
    _r("Dew/frost point", "7–8", "6–7", "F32", "°C", "Calculated dew/frost-point temperature."),
    _r("Absolute humidity", "9–10", "8–9", "F32", "g/m³", "Calculated absolute humidity."),
    _r("Mixing ratio", "11–12", "10–11", "F32", "g/kg", "Calculated water-vapor mixing ratio."),
    _r("Wet-bulb temperature", "13–14", "12–13", "F32", "°C", "Calculated wet-bulb temperature."),
    _r("Enthalpy", "15–16", "14–15", "F32", "kJ/kg", "Calculated moist-air enthalpy."),
    _r("Device status", "513", "512", "S16", "", "Overall device status; zero is expected when healthy.", "Device status"),
    _r("Error code", "514–515", "513–514", "S32", "", "Current error code; zero is expected when healthy."),
    _r("RH measurement status", "518", "517", "S16", "", "Relative-humidity channel status; zero is expected when healthy."),
    _r("Temperature status", "519", "518", "S16", "", "Temperature channel status; zero is expected when healthy."),
)


WATTNODE_REGISTERS = (
    _r("Energy total", "1001–1002", "1000–1001", "F32 HL", "kWh", "Total net bidirectional active energy.", "Total energy"),
    _r("Active power total", "1009–1010", "1008–1009", "F32 HL", "W", "Sum of active power across enabled meter elements.", "Total active power"),
    _r("Active power 1", "1011–1012", "1010–1011", "F32 HL", "W", "Active power for meter element 1."),
    _r("Active power 2", "1013–1014", "1012–1013", "F32 HL", "W", "Active power for meter element 2."),
    _r("Active power 3", "1015–1016", "1014–1015", "F32 HL", "W", "Active power for meter element 3."),
    _r("Voltage AN", "1019–1020", "1018–1019", "F32 HL", "V", "RMS line-A to neutral voltage."),
    _r("Voltage BN", "1021–1022", "1020–1021", "F32 HL", "V", "RMS line-B to neutral voltage."),
    _r("Voltage CN", "1023–1024", "1022–1023", "F32 HL", "V", "RMS line-C to neutral voltage."),
    _r("Frequency", "1033–1034", "1032–1033", "F32 HL", "Hz", "Measured AC line frequency.", "Line frequency"),
    _r("Current 1", "1163–1164", "1162–1163", "F32 HL", "A", "RMS current for CT input 1."),
    _r("Current 2", "1165–1166", "1164–1165", "F32 HL", "A", "RMS current for CT input 2."),
    _r("Current 3", "1167–1168", "1166–1167", "F32 HL", "A", "RMS current for CT input 3."),
    _r("Integer frequency", "1221", "1220", "S16", "0.01 Hz", "Simple first-contact measurement that does not require float decoding."),
    _r("CT amps global", "1603", "1602", "U16", "A", "Writes the rated current to all three CT channels.", access="R/W"),
    _r("CT amps 1", "1604", "1603", "U16", "A", "Rated primary current for CT input 1.", access="R/W"),
    _r("CT amps 2", "1605", "1604", "U16", "A", "Rated primary current for CT input 2.", access="R/W"),
    _r("CT amps 3", "1606", "1605", "U16", "A", "Rated primary current for CT input 3.", access="R/W"),
    _r("CT directions", "1607", "1606", "U16", "bitfield", "Optionally invert individual CT orientations.", access="R/W"),
    _r("Gain adjust 1", "1612", "1611", "U16", "1/10000", "CT1 calibration gain; do not change without traceable calibration data.", access="R/W"),
    _r("Gain adjust 2", "1613", "1612", "U16", "1/10000", "CT2 calibration gain; do not change without traceable calibration data.", access="R/W"),
    _r("Gain adjust 3", "1614", "1613", "U16", "1/10000", "CT3 calibration gain; do not change without traceable calibration data.", access="R/W"),
    _r("Phase adjust 1", "1615", "1614", "S16", "0.001°", "CT1 phase correction; preserve unless CT characterization requires it.", access="R/W"),
    _r("Phase adjust 2", "1616", "1615", "S16", "0.001°", "CT2 phase correction; preserve unless CT characterization requires it.", access="R/W"),
    _r("Phase adjust 3", "1617", "1616", "S16", "0.001°", "CT3 phase correction; preserve unless CT characterization requires it.", access="R/W"),
    _r("Current integer scale", "1622", "1621", "U16", "counts", "Full-scale count used by integer current registers; default 20000.", access="R/W"),
    _r("Meter config 1", "1624", "1623", "U16", "code", "Voltage mapping for meter element 1.", access="R/W"),
    _r("Meter config 2", "1625", "1624", "U16", "code", "Voltage mapping for meter element 2.", access="R/W"),
    _r("Meter config 3", "1626", "1625", "U16", "code", "Voltage mapping for meter element 3.", access="R/W"),
    _r("Change counter", "1628", "1627", "U16", "writes", "Persistent count of configuration changes.", access="R"),
    _r("Nominal CT volts 1", "1629–1630", "1628–1629", "F32 HL", "Vac", "Full-scale output voltage for CT input 1; normally 0.33333 Vac.", access="R/W"),
    _r("Nominal CT volts 2", "1631–1632", "1630–1631", "F32 HL", "Vac", "Full-scale output voltage for CT input 2; normally 0.33333 Vac.", access="R/W"),
    _r("Nominal CT volts 3", "1633–1634", "1632–1633", "F32 HL", "Vac", "Full-scale output voltage for CT input 3; normally 0.33333 Vac.", access="R/W"),
    _r("Connection type", "1636", "1635", "U16", "code", "Shortcut defining common service and meter-element mappings.", access="R/W"),
    _r("Address", "1652", "1651", "U16", "", "Pending/current Modbus slave address.", access="R/W"),
    _r("Baud rate", "1653", "1652", "U16", "code", "Configured Modbus baud-rate code.", access="R/W"),
    _r("Parity mode", "1654", "1653", "U16", "code", "0=8N1, 1=8E1, 2=8N2.", access="R/W"),
    _r("Serial number", "1701–1702", "1700–1701", "U32 HL", "", "Serial number printed on the meter label.", "Serial number"),
    _r("Model", "1707", "1706", "U16", "code", "Model code 530 identifies a WattNode meter module.", "Model code"),
    _r("Firmware version", "1708", "1707", "U16", "", "WND module firmware version in the 10xx family.", "Firmware"),
    _r("Options", "1709", "1708", "U16", "bitfield", "Factory options bitfield."),
    _r("Error status 1–8", "1716–1723", "1715–1722", "U16[8]", "", "Newest-to-oldest diagnostic errors and events."),
    _r("Factory CT amps 1–3", "1724–1726", "1723–1725", "U16[3]", "A", "Read-only CT ratings originally ordered from the factory."),
    _r("Factory address", "1728", "1727", "U16", "", "Read-only factory Modbus address option."),
    _r("Factory baud", "1729", "1728", "U16", "code", "Read-only factory baud-rate option."),
    _r("Factory parity", "1730", "1729", "U16", "code", "Read-only factory parity option."),
    _r("Factory config lock", "1737", "1736", "U16", "bool", "Reports whether configuration was factory locked."),
)


REGISTER_MAPS = {
    "dpt146": DPT146_REGISTERS,
    "hmd65": HMD65_REGISTERS,
    "wattnode": WATTNODE_REGISTERS,
}
