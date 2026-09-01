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
        status="Connected · identity pending",
        description="The shared Synetica STM32 USB interface is present, but the product banner has not yet been read.",
        color="#7D8597",
        facts=(("USB identity", "0483:5740"), ("Safety", "Modbus polling blocked")),
        help_setup=(
            "Close other copies of the configurator and terminal programs that may hold the COM port.",
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
        help_setup=(
            "Power the bridge from 12–24 VDC; USB is configuration-only.",
            "Connect the Modbus instrument to RS485 IN and keep only one active master on the bus.",
            "Back up the existing table before selecting or importing a configuration.",
            "After import, use Read All Data Points and require zero exceptions.",
        ),
        help_troubleshooting=(
            "If no COM port appears, reconnect USB and refresh device discovery.",
            "If all reads time out, check slave ID, serial format, common, and A/B polarity.",
            "Implausible float values usually indicate the wrong PDU address or word order.",
            "A row with Slave ID 0 deletes that item; restore from a backup or pre-made table.",
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
        help_setup=(
            "Use physical connector II for RS-485/Modbus; CH1 and CH2 are analog-output labels.",
            "The validated unit uses slave 1 at 19200 baud, 8N2.",
            "Select the validated DPT146 table and verify all eight points before deployment.",
            "Expected healthy values are fault 1, online 1, and error code 0.",
        ),
        help_troubleshooting=(
            "No response: confirm connector II, supply power, common wiring, and slave ID 1.",
            "Plausible but wrong readings: use zero-based PDU addresses and HL word order.",
            "Do not connect COM3 as another active master while the bridge is energized.",
            "If status is unhealthy, record raw status registers before changing configuration.",
        ),
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
        help_setup=(
            "Photograph the protocol, address, bitrate, parity, and termination DIP switches first.",
            "Confirm Modbus RTU mode and change the TSV slave ID to match the address switches.",
            "Start with 19200 8N1 only when the physical DIP positions select those settings.",
            "Import the 12-point test table and perform read-only validation.",
        ),
        help_troubleshooting=(
            "No response: verify Modbus mode rather than BACnet mode and confirm the DIP address.",
            "Bad float values: test HH first, then HL without changing register addresses.",
            "Status values should normally be zero; save non-zero bitmasks for diagnosis.",
            "Do not perform calibration writes during initial commissioning.",
        ),
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
            ("First contact", "19200 8N1 · IDs 1 then 127"),
            ("CT/service", "Installation-specific"),
        ),
        artifact=ROOT / "artifacts/device-profiles/to-test/wattnode-wnd-m1-mb.yaml",
        help_setup=(
            "Confirm the label says WND-M1-MB and record firmware and factory option text.",
            "Record service type, voltage mapping, and every CT rating before configuration.",
            "Connect directly with the bridge isolated; try 19200 8N1 at ID 1, then legacy ID 127.",
            "Read identity and configuration first. Do not write until the snapshot is saved.",
            "Use the 12-point float table first to avoid integer current and power scaling.",
        ),
        help_troubleshooting=(
            "No response: inspect front-label communication options and verify A−, B+, common, and bus termination.",
            "Wrong sign or phase: inspect CT direction and voltage-to-CT mapping before adjustment.",
            "For a 250 A CT and CurrentIntScale 20000, integer resolution is 0.0125 A/count.",
            "Do not guess gain or phase calibration values; preserve the meter's existing values.",
        ),
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
        help_setup=(
            "Connect the internal configuration USB port; the device emits an identity banner before login.",
            "Confirm the banner says North American Hybrid FSB #1 on 915 MHz for normal US deployments.",
            "Use the last four DevEUI characters as the factory-default console password unless it has been changed.",
            "After login, use Quick Start for JoinEUI/AppEUI, AppKey, and transmit interval; reboot to apply changes.",
        ),
        help_troubleshooting=(
            "If COM appears but no banner is read, close other terminal programs and reconnect the configuration USB cable.",
            "Do not infer the exact model from VID/PID: multiple Synetica products share the STM32 USB identity.",
            "A region mismatch cannot be corrected by network-server settings alone; select a compatible device/firmware region profile.",
            "Installed gas, sound, and particle channels are option-dependent; inventory them from Live Readings before commissioning.",
        ),
    ),
    "adapter": DeviceDefinition(
        key="adapter",
        name="Serial transport",
        subtitle="USB/RS-485 candidate",
        kind="adapter",
        status="Transport detected",
        description="Candidate direct Modbus transport. The attached instrument is identified independently from protocol responses.",
        color="#7D8597",
        facts=(
            ("Port selection", "Dynamic USB discovery"),
            ("Identity rule", "Modbus fingerprint"),
            ("Use", "Direct isolated validation"),
            ("Safety", "Bridge off or isolated"),
        ),
        artifact=ROOT / "docs/reference/usb-comi-tb-manual.pdf",
        detect_tokens=("USB-COMI", "USB COMI", "FTDI"),
        help_setup=(
            "Power down or isolate the bridge master before direct polling.",
            "Connect D−, D+, and common according to the adapter terminal labels.",
            "Set the adapter for two-wire RS-485 and match the instrument serial format.",
        ),
        help_troubleshooting=(
            "If no serial transport appears, reconnect USB and inspect Windows Device Manager.",
            "If reads time out, check A/B naming differences and common wiring.",
            "Never diagnose by transmitting from both the bridge and adapter at once.",
        ),
    ),
}


PREMADE_CONFIGS = {
    "dpt146": ROOT / "artifacts/bridge-config/vaisala-dpt146-golden.tsv",
    "hmd65": ROOT / "artifacts/bridge-config/hmd65-documentation-test.tsv",
    "wattnode": ROOT / "artifacts/bridge-config/wattnode-wnd-m1-mb-documentation-test.tsv",
}
