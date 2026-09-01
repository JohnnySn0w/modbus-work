from __future__ import annotations

import re
import os
import subprocess
import time
from dataclasses import dataclass, field
from typing import Any

from .discovery import PortInfo


ANSI_ESCAPE = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]")


def clean_terminal_text(text: str) -> str:
    return ANSI_ESCAPE.sub("", text).replace("\x00", "")


def normalize_dev_eui(value: str) -> str:
    normalized = re.sub(r"[^0-9a-fA-F]", "", value).lower()
    if len(normalized) != 16:
        raise ValueError("DevEUI must contain exactly 16 hexadecimal characters")
    return normalized


def default_console_password(dev_eui: str) -> str:
    """Synetica enLink factory serial login: final four DevEUI hex digits."""
    return normalize_dev_eui(dev_eui)[-4:]


@dataclass(frozen=True)
class EnlinkProbeResult:
    key: str
    port: str
    confidence: str
    reason: str
    display_name: str
    firmware: str
    firmware_code: str
    region: str
    dev_eui: str
    derived_login: str
    serial_format: str = "USB configuration console"
    slave_id: str = "n/a"
    readings: dict[str, tuple[float | int | str, str]] = field(default_factory=dict)


def parse_enlink_banner(text: str, port: str) -> EnlinkProbeResult | None:
    clean = clean_terminal_text(text)
    if "Synetica - enLink" not in clean:
        return None
    fields: dict[str, str] = {}
    for label in ("Region", "Firmware Code", "Firmware Ver", "Description", "DevEui",
                  "Model Number", "Model No", "Model No.", "Model Name"):
        match = re.search(rf"^{re.escape(label)}:\s*(.+?)\s*$", clean,
                          re.IGNORECASE | re.MULTILINE)
        if match:
            fields[label] = match.group(1).strip()
    firmware_code = fields.get("Firmware Code", "")
    description = fields.get("Description", "")
    model_number = fields.get("Model Number", fields.get("Model No", fields.get("Model No.", "")))
    model_name = fields.get("Model Name", "")
    if firmware_code.startswith("FW-AQ-") or "Air Quality" in description:
        key = "iaq_plus"
        display_name = "Synetica enLink IAQ Plus"
    elif (model_number.upper() == "ENL-MOD-32" or "MOD" in firmware_code.upper()
          or "MODBUS" in description.upper() or "MODBUS" in model_name.upper()):
        key = "bridge"
        display_name = "Synetica ENL-MOD-32"
    else:
        return None
    dev_eui = fields.get("DevEui", "Unknown")
    try:
        derived_login = default_console_password(dev_eui)
    except ValueError:
        return None
    return EnlinkProbeResult(
        key=key,
        port=port,
        confidence="high",
        reason="Synetica product family, firmware code, region, and DevEUI matched the USB banner.",
        display_name=display_name,
        firmware=fields.get("Firmware Ver", "Unknown"),
        firmware_code=firmware_code or model_number or "Unknown",
        region=fields.get("Region", "Unknown"),
        dev_eui=dev_eui,
        derived_login=derived_login,
        readings={"Device state": ("Configuration console available", "")},
    )


def probe_enlink_console(port: PortInfo, timeout: float = 1.0) -> EnlinkProbeResult | None:
    """Read the unauthenticated banner emitted when an enLink USB console opens."""
    import serial

    result: EnlinkProbeResult | None = None
    try:
        connection = serial.Serial(
            port=None, baudrate=115200, bytesize=8, parity="N", stopbits=1,
            timeout=0.1, write_timeout=0.1, rtscts=False, dsrdtr=False,
        )
        connection.dtr = True
        connection.rts = False
        connection.port = port.port
        connection.open()
        with connection:
            deadline = time.monotonic() + timeout
            chunks: list[bytes] = []
            while time.monotonic() < deadline:
                waiting = connection.in_waiting
                chunk = connection.read(waiting or 1)
                if chunk:
                    chunks.append(chunk)
                    if b"Password:" in b"".join(chunks):
                        break
            # Some enLink firmware emits its banner only after terminal input.
            # A carriage return wakes the console without authenticating or
            # changing configuration.
            if not chunks:
                connection.write(b"\r")
                connection.flush()
                deadline = time.monotonic() + 2.0
                while time.monotonic() < deadline:
                    waiting = connection.in_waiting
                    chunk = connection.read(waiting or 1)
                    if chunk:
                        chunks.append(chunk)
                        if b"Password:" in b"".join(chunks):
                            break
            result = parse_enlink_banner(b"".join(chunks).decode("utf-8", errors="replace"), port.port)
    except (OSError, ValueError):
        pass
    if result or os.name != "nt":
        return result

    root = __import__("pathlib").Path(__file__).resolve().parents[1]
    script = root / "tools" / "enlink_probe_banner.ps1"
    try:
        completed = subprocess.run(
            ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", str(script),
             "-PortName", port.port],
            capture_output=True, text=True, timeout=8,
            creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if completed.returncode:
        return None
    return parse_enlink_banner(completed.stdout, port.port)


@dataclass(frozen=True)
class EnlinkConsoleSnapshot:
    identity: EnlinkProbeResult
    main_menu: str
    pages: dict[str, str]


def parse_sensor_readings(text: str) -> dict[str, tuple[float | int | str, str]]:
    clean = clean_terminal_text(text).replace("?C", "°C")
    if "Sensor Readings (Page 1)" not in clean:
        return {}

    def number(pattern: str) -> float | None:
        match = re.search(pattern, clean, re.IGNORECASE | re.MULTILINE)
        return float(match.group(1)) if match else None

    base_match = re.search(
        r"Sensor Readings \(Page 1\):.*?^\s*Temperature\s+(-?\d+(?:\.\d+)?)\s*[?°]?C",
        clean, re.IGNORECASE | re.MULTILINE | re.DOTALL,
    )
    base_temperature = float(base_match.group(1)) if base_match else None
    humidity = number(r"^\s*Humidity\s+(\d+(?:\.\d+)?)\s*%")
    pressure = number(r"^\s*Pressure\s+(\d+(?:\.\d+)?)\s*mbar")
    co2e = number(r"^\s*CO2e Estimate\s+(\d+(?:\.\d+)?)\s*ppm")
    bvoc = number(r"^\s*bVOC Estimate\s+(\d+(?:\.\d+)?)\s*ppm")
    iaq_match = re.search(r"^\s*IAQ \[Accuracy\]\s+(\d+)\s+IAQ\s+\[(\d+:[^\]]+)\]",
                          clean, re.IGNORECASE | re.MULTILINE)
    gss_match = re.search(r"-- CO2 Sensor \(GSS\).*?^\s*Reading\s+(\d+(?:\.\d+)?)\s*ppm",
                          clean, re.IGNORECASE | re.MULTILINE | re.DOTALL)

    readings: dict[str, tuple[float | int | str, str]] = {}
    for label, value, unit in (
        ("Temperature", base_temperature, "°C"),
        ("Relative humidity", humidity, "%RH"),
        ("Pressure", pressure, "mbar"),
        ("CO₂ equivalent", co2e, "ppm"),
        ("bVOC estimate", bvoc, "ppm"),
    ):
        if value is not None:
            readings[label] = (value, unit)
    if iaq_match:
        readings["IAQ"] = (int(iaq_match.group(1)), iaq_match.group(2))
    if gss_match:
        readings["CO₂"] = (float(gss_match.group(1)), "ppm")
    return readings


def capture_page_windows(port: str, page: str, redact_secrets: bool = True) -> str:
    if page not in {"quick_start", "radio", "configure", "configure_page2"}:
        raise ValueError(f"Unsupported enLink page: {page}")
    root = __import__("pathlib").Path(__file__).resolve().parents[1]
    script = root / "tools" / "enlink_read_page.ps1"
    command = [
        "powershell", "-NoProfile", "-ExecutionPolicy", "Bypass",
        "-File", str(script), "-PortName", port, "-Page", page,
    ]
    if redact_secrets:
        command.append("-RedactSecrets")
    completed = subprocess.run(command, capture_output=True, text=True, timeout=35)
    if completed.returncode:
        raise RuntimeError(completed.stderr.strip() or completed.stdout.strip())
    return completed.stdout


def read_live_values_windows(port: str) -> dict[str, tuple[float | int | str, str]]:
    return parse_sensor_readings(capture_page_windows(port, "configure"))


class EnlinkConsoleSession:
    """Small authenticated console transport for technician-initiated work.

    Background discovery must continue to use ``probe_enlink_console`` and
    stop at the password prompt. This class is for explicit read/configure
    actions after the operator selects the connected device.
    """

    def __init__(self, port: str, timeout: float = 0.1) -> None:
        self.port = port
        self.timeout = timeout
        self.connection: Any | None = None
        self.identity: EnlinkProbeResult | None = None

    def __enter__(self) -> "EnlinkConsoleSession":
        import serial

        self.connection = serial.Serial(
            port=None, baudrate=115200, bytesize=8, parity="N", stopbits=1,
            timeout=self.timeout, write_timeout=0.5, rtscts=False, dsrdtr=False,
        )
        self.connection.dtr = True
        self.connection.rts = False
        self.connection.port = self.port
        self.connection.open()
        initial = self.read_for(1.5)
        self.identity = parse_enlink_banner(initial, self.port)
        if self.identity is None:
            raise RuntimeError("The connected serial device did not emit a recognized enLink banner")
        if "enlink Main Menu" not in clean_terminal_text(initial):
            if "Password:" not in clean_terminal_text(initial):
                self.connection.write(b"\r")
                initial += self.read_for(1.0)
            self.connection.write((self.identity.derived_login + "\r").encode("ascii"))
            initial += self.read_for(2.0)
        if "enlink Main Menu" not in clean_terminal_text(initial):
            raise RuntimeError("The derived enLink login was not accepted by the serial console")
        self._main_menu = clean_terminal_text(initial)
        return self

    def __exit__(self, _type, _value, _traceback) -> None:
        if self.connection is not None:
            self.connection.close()
            self.connection = None

    def read_for(self, seconds: float) -> str:
        if self.connection is None:
            raise RuntimeError("Console is not open")
        deadline = time.monotonic() + seconds
        chunks: list[bytes] = []
        while time.monotonic() < deadline:
            waiting = self.connection.in_waiting
            chunk = self.connection.read(waiting or 1)
            if chunk:
                chunks.append(chunk)
            else:
                time.sleep(0.02)
        return b"".join(chunks).decode("utf-8", errors="replace")

    def command(self, value: str, wait: float = 1.5) -> str:
        if self.connection is None:
            raise RuntimeError("Console is not open")
        if not value or "\r" in value or "\n" in value:
            raise ValueError("Console commands must be a non-empty single-line value")
        self.connection.reset_input_buffer()
        self.connection.write((value + "\r").encode("ascii"))
        self.connection.flush()
        return clean_terminal_text(self.read_for(wait))

    @property
    def main_menu(self) -> str:
        return self._main_menu


def capture_read_only_snapshot(port: str) -> EnlinkConsoleSnapshot:
    if os.name == "nt":
        pages = {
            page: capture_page_windows(port, page, redact_secrets=False)
            for page in ("quick_start", "radio", "configure")
        }
        identity = parse_enlink_banner(next(iter(pages.values())), port)
        if identity is None:
            raise RuntimeError("Unable to parse enLink identity from captured console pages")
        return EnlinkConsoleSnapshot(identity, "", pages)
    pages: dict[str, str] = {}
    with EnlinkConsoleSession(port) as session:
        assert session.identity is not None
        for key, command in (("quick_start", "Q"), ("radio", "L"), ("configure", "C")):
            pages[key] = session.command(command)
            session.command("X", wait=0.8)
        return EnlinkConsoleSnapshot(session.identity, session.main_menu, pages)
