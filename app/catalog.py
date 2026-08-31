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


DEVICES: dict[str, DeviceDefinition] = {
    "bridge": DeviceDefinition(
        key="bridge",
        name="Polygon ExactAire-E5",
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
    ),
    "dpt146": DeviceDefinition(
        key="dpt146",
        name="Vaisala DPT146",
        subtitle="Dewpoint and pressure transmitter",
        kind="humidity",
        status="Golden configuration",
        description="Validated eight-point profile connected through physical connector II (RS-485/Modbus).",
        color="#38A3A5",
        readouts=(
            Readout("Temperature", "22.89", "°C"),
            Readout("Dew / frost point", "7.36", "°C"),
            Readout("Atmospheric dew point", "7.41", "°C"),
            Readout("Moisture", "10,276", "ppmv"),
            Readout("Absolute pressure", "1.0095", "bara"),
            Readout("Device health", "Online", quality="Fault 1 · Error 0"),
        ),
        facts=(
            ("Slave", "1"),
            ("Serial", "19200 · 8N2"),
            ("Connector", "II — RS-485"),
            ("Word order", "HL for 32-bit values"),
        ),
        artifact=ROOT / "artifacts/bridge-config/vaisala-dpt146-manifest.yaml",
    ),
    "hmd65": DeviceDefinition(
        key="hmd65",
        name="Vaisala HMD65",
        subtitle="Humidity and temperature transmitter",
        kind="humidity",
        status="Ready to test",
        description="Documentation-derived 12-point profile. Hardware validation is still required.",
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
    ),
    "wattnode": DeviceDefinition(
        key="wattnode",
        name="WattNode WND-M1-MB",
        subtitle="Module for Modbus",
        kind="meter",
        status="Ready to test",
        description="Documentation-derived 12-point float profile for the specific WND-M1-MB module.",
        color="#E0A458",
        readouts=(
            Readout("Total active power", "—", "W", "Awaiting hardware"),
            Readout("Total energy", "—", "kWh", "Awaiting hardware"),
            Readout("Line frequency", "—", "Hz", "Awaiting hardware"),
            Readout("CT currents", "—", "A", "Awaiting hardware"),
        ),
        facts=(
            ("Manual", "WND-M1-MB-Ref-1.10"),
            ("32-bit order", "Low word first / HL"),
            ("Points", "12 native float values"),
            ("CT scale", "Installation-specific"),
        ),
        artifact=ROOT / "artifacts/device-profiles/to-test/wattnode-wnd-m1-mb.yaml",
    ),
    "adapter": DeviceDefinition(
        key="adapter",
        name="USB-COMi-TB",
        subtitle="USB to RS-485 adapter",
        kind="adapter",
        status="Bench interface",
        description="Direct read-only Modbus interface. Never operate it as a second master on the live bridge bus.",
        color="#7D8597",
        facts=(
            ("Known port", "COM3"),
            ("Use", "Direct isolated validation"),
            ("Safety", "Bridge off or isolated"),
        ),
        artifact=ROOT / "docs/reference/usb-comi-tb-manual.pdf",
        detect_tokens=("USB-COMI", "USB COMI", "FTDI"),
    ),
}

