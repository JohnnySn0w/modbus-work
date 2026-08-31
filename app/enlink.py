from __future__ import annotations

import re
import time
from dataclasses import dataclass, field

from .discovery import PortInfo


ANSI_ESCAPE = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]")


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
    clean = ANSI_ESCAPE.sub("", text)
    if "Synetica - enLink" not in clean:
        return None
    fields: dict[str, str] = {}
    for label in ("Region", "Firmware Code", "Firmware Ver", "Description", "DevEui"):
        match = re.search(rf"^{re.escape(label)}:\s*(.+?)\s*$", clean,
                          re.IGNORECASE | re.MULTILINE)
        if match:
            fields[label] = match.group(1).strip()
    firmware_code = fields.get("Firmware Code", "")
    description = fields.get("Description", "")
    if firmware_code.startswith("FW-AQ-") or "Air Quality" in description:
        key = "iaq_plus"
        display_name = "Synetica enLink IAQ Plus"
    elif "MOD" in firmware_code.upper() or "MODBUS" in description.upper():
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
        firmware_code=firmware_code or "Unknown",
        region=fields.get("Region", "Unknown"),
        dev_eui=dev_eui,
        derived_login=derived_login,
        readings={"Device state": ("Configuration console available", "")},
    )


def probe_enlink_console(port: PortInfo, timeout: float = 1.0) -> EnlinkProbeResult | None:
    """Read the unauthenticated banner emitted when an enLink USB console opens."""
    import serial

    try:
        with serial.Serial(
            port.port, 115200, bytesize=8, parity="N", stopbits=1,
            timeout=0.1, write_timeout=0.1,
        ) as connection:
            connection.dtr = True
            connection.rts = True
            deadline = time.monotonic() + timeout
            chunks: list[bytes] = []
            while time.monotonic() < deadline:
                waiting = connection.in_waiting
                chunk = connection.read(waiting or 1)
                if chunk:
                    chunks.append(chunk)
                    if b"Password:" in b"".join(chunks):
                        break
            return parse_enlink_banner(b"".join(chunks).decode("utf-8", errors="replace"), port.port)
    except (OSError, ValueError):
        return None
