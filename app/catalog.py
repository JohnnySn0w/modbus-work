from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class Readout:
    label: str
    value: str
    unit: str = ""
    quality: str = "Last validated"


@dataclass(frozen=True)
class DeviceDefinition:
    key: str
    name: str
    subtitle: str
    kind: str
    status: str
    description: str
    color: str
    readouts: tuple[Readout, ...] = ()
    facts: tuple[tuple[str, str], ...] = ()
    artifact: Path | None = None
    detect_tokens: tuple[str, ...] = field(default_factory=tuple)
    help_setup: tuple[str, ...] = field(default_factory=tuple)
    help_troubleshooting: tuple[str, ...] = field(default_factory=tuple)


DEVICES: dict[str, DeviceDefinition] = {
    "synetica_usb": DeviceDefinition(
        key="synetica_usb",
        name="Synetica USB device",
        subtitle="Waiting for console identity banner",
        kind="bridge",
        status="Connected · device details not yet read",
        description="The shared Synetica STM32 USB interface is present, but the product banner has not yet been read.",
        color="#7D8597",
        facts=(("USB identity", "0483:5740"), ("Safety", "Modbus polling blocked")),
        help_setup=(
            "Close other copies of the configurator and terminal programs that may hold the serial port.",
            "Confirm the Synetica device is powered, then reconnect its configuration USB cable.",
            "Refresh and wait for the unauthenticated console banner to identify the exact product.",
        ),
        help_troubleshooting=(
            "The STM32 VID/PID is shared by bridges and sensors, so USB metadata alone is not an exact identity.",
            "If identity remains pending, power-cycle the device and reconnect USB with only one GUI running.",
            "Direct RS-485 polling stays disabled while this possible bridge master is connected.",
        ),
    ),
    "bridge": DeviceDefinition(
        key="bridge",
        name="Modbus Bridge",
        subtitle="Synetica ENL-MOD-32 bridge",
        kind="bridge",
        status="Validated",
        description="32-point Modbus RTU master and North American 915 MHz LoRaWAN bridge.",
        color="#4C78A8",
        readouts=(
            Readout("Configured points", "8 / 32", quality="Restored after reboot"),
            Readout("Successful reads", "8 / 0", "OK / exceptions", "Validated"),
            Readout("Transmit interval", "15", "minutes", "Configured"),
            Readout("LoRaWAN", "Joined", quality="Validated in Loriot"),
        ),
        facts=(
            ("Firmware", "3.6"),
            ("Modbus", "19200 · 8N2"),
            ("Radio", "NA hybrid 915 MHz"),
            ("Capacity", "32 data points"),
        ),
        artifact=ROOT / "artifacts/bridge-config/README.md",
        detect_tokens=("VID_0483&PID_5740", "STM32", "ENL-MOD-32"),
        help_setup=(
            "Power the bridge from 12–24 volts direct current; USB is configuration-only.",
            "Connect the Modbus instrument to RS485 IN and keep only one active master on the bus.",
            "Back up the existing table before selecting or importing a configuration.",
            "After import, use Read All Data Points and require zero exceptions.",
        ),
        help_troubleshooting=(
            "If no serial port appears, reconnect USB and refresh device discovery.",
            "If all reads time out, check slave address, serial format, common, and A/B polarity.",
            "Implausible float values usually indicate the wrong transmitted register address or word order.",
            "A row with Slave address 0 deletes that item; restore from a backup or pre-made table.",
        ),
    ),
    "dpt146": DeviceDefinition(
        key="dpt146",
        name="Vaisala DPT146",
        subtitle="Dewpoint and pressure transmitter",
        kind="humidity",
        status="Validated configuration",
        description="Validated eight-point profile connected through physical connector II (RS-485/Modbus).",
        color="#38A3A5",
        readouts=(
            Readout("Temperature", "Not read", "", "Use Refresh live readings"),
            Readout("Dew / frost point", "Not read", "", "Use Refresh live readings"),
            Readout("Atmospheric dew point", "Not read", "", "Use Refresh live readings"),
            Readout("Moisture", "Not read", "", "Use Refresh live readings"),
            Readout("Absolute pressure", "Not read", "", "Use Refresh live readings"),
            Readout("Device health", "Not read", quality="Use Refresh live readings"),
        ),
        facts=(
            ("Slave", "1"),
            ("Serial", "19200 · 8N2"),
            ("Connector", "II — RS-485"),
            ("Word order", "HL for 32-bit values"),
        ),
        artifact=ROOT / "artifacts/bridge-config/vaisala-dpt146-manifest.yaml",
        help_setup=(
            "Use physical connector II for RS-485/Modbus; CH1 and CH2 are analog-output labels.",
            "The validated unit uses slave 1 at 19200 baud, 8 data bits, no parity and 2 stop bits.",
            "Select the validated DPT146 table and verify all eight points before deployment.",
            "Expected healthy values are fault 1, online 1, and error code 0.",
        ),
        help_troubleshooting=(
            "No response: confirm connector II, supply power, common wiring and the configured slave address. Address 1 was used in the recorded test; other installations may differ.",
            "Plausible but wrong readings: use zero-based transmitted register addresses and the word order specified by the reviewed profile.",
            "Switch the Modbus Bridge off using its hardware switch before reading through the USB adapter on the same bus. External power can remain connected.",
            "If status is unhealthy, record raw status registers before changing configuration.",
        ),
    ),
    "hmd65": DeviceDefinition(
        key="hmd65",
        name="Vaisala HMD65",
        subtitle="Humidity and temperature transmitter",
        kind="humidity",
        status="Ready to test",
        description="12-register configuration based on the manufacturer documentation. Testing with a connected sensor is pending.",
        color="#73A942",
        readouts=(
            Readout("Relative humidity", "—", "%RH", "Awaiting hardware"),
            Readout("Temperature", "—", "°C", "Awaiting hardware"),
            Readout("Dew point", "—", "°C", "Awaiting hardware"),
            Readout("Device status", "—", quality="Awaiting hardware"),
        ),
        facts=(
            ("Candidate baud", "19200"),
            ("Candidate format", "8N1"),
            ("Points", "8 measurements + 4 status"),
            ("Open check", "Float word order"),
        ),
        artifact=ROOT / "artifacts/device-profiles/to-test/vaisala-hmd65.yaml",
        help_setup=("TBD",),
        help_troubleshooting=("TBD",),
    ),
    "wattnode": DeviceDefinition(
        key="wattnode",
        name="WattNode WND-M1-MB",
        subtitle="Module for Modbus",
        kind="meter",
        status="Ready to test",
        description="12-register configuration for the WND-M1-MB, using floating-point measurements. Testing with a connected meter is pending.",
        color="#E0A458",
        readouts=(
            Readout("Total active power", "—", "W", "Awaiting hardware"),
            Readout("Total energy", "—", "kWh", "Awaiting hardware"),
            Readout("Line frequency", "—", "Hz", "Awaiting hardware"),
            Readout("Current transformer currents", "—", "A", "Awaiting hardware"),
        ),
        facts=(
            ("Manual", "WND-M1-MB-Ref-1.10"),
            ("32-bit order", "Low word first / HL"),
            ("Points", "12 native float values"),
            ("First contact", "19200 8N1 · IDs 1 then 127"),
            ("CT/service", "Installation-specific"),
        ),
        artifact=ROOT / "artifacts/device-profiles/to-test/wattnode-wnd-m1-mb.yaml",
        help_setup=("TBD",),
        help_troubleshooting=("TBD",),
    ),
    "iaq_plus": DeviceDefinition(
        key="iaq_plus",
        name="Synetica enLink IAQ Plus",
        subtitle="LoRaWAN indoor air-quality monitor",
        kind="air_quality",
        status="Connected · identified",
        description="Direct LoRaWAN sensor with a USB configuration console; observed part 003-ADZ-301.",
        color="#6D5BD0",
        readouts=(
            Readout("Device state", "Configuration console available", quality="Live USB banner"),
            Readout("Temperature", "—", "°C", "Open Live Readings after login"),
            Readout("Relative humidity", "—", "%RH", "Open Live Readings after login"),
            Readout("Pressure", "—", "mbar", "Open Live Readings after login"),
            Readout("CO₂", "—", "ppm", "Open Live Readings after login"),
            Readout("CO₂ equivalent", "—", "ppm", "Open Live Readings after login"),
            Readout("bVOC estimate", "—", "ppm", "Open Live Readings after login"),
            Readout("IAQ", "—", quality="Open Live Readings after login"),
        ),
        facts=(
            ("Observed part", "003-ADZ-301"),
            ("Firmware", "FW-AQ-VCP+ · 5.06"),
            ("Default region", "US915 Hybrid FSB #1"),
            ("Configuration", "USB virtual COM console"),
        ),
        artifact=ROOT / "artifacts/device-profiles/synetica-enlink-iaq-plus.yaml",
        detect_tokens=("FW-AQ-VCP+", "Air Quality - Environmental Sensors"),
        help_setup=("TBD",),
        help_troubleshooting=("TBD",),
    ),
    "adapter": DeviceDefinition(
        key="adapter",
        name="USB to RS-485 adapter",
        subtitle="Direct Modbus connection",
        kind="adapter",
        status="USB adapter detected",
        description="Connects this computer to Modbus sensors through RS-485. Readings come from the connected sensors, not the adapter.",
        color="#7D8597",
        facts=(
            ("Port selection", "Dynamic USB discovery"),
            ("Identity rule", "Modbus fingerprint"),
            ("Use", "Direct isolated validation"),
            ("Safety", "Bridge off or isolated"),
        ),
        artifact=ROOT / "docs/reference/usb-comi-tb-manual.pdf",
        detect_tokens=("USB-COMI", "USB COMI", "FTDI"),
        help_setup=("TBD",),
        help_troubleshooting=("TBD",),
    ),
}


PREMADE_CONFIGS = {
    "dpt146": ROOT / "artifacts/bridge-config/vaisala-dpt146-validated.tsv",
    "hmd65": ROOT / "artifacts/bridge-config/hmd65-documentation-test.tsv",
    "wattnode": ROOT / "artifacts/bridge-config/wattnode-wnd-m1-mb-documentation-test.tsv",
}
