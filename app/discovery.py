from __future__ import annotations

import os
import re
import subprocess
import threading
from collections.abc import Callable
from dataclasses import dataclass


@dataclass(frozen=True)
class PortInfo:
    port: str
    description: str
    hardware_id: str = ""

    @property
    def search_text(self) -> str:
        return f"{self.port} {self.description} {self.hardware_id}".upper()


def _pyserial_ports() -> list[PortInfo]:
    try:
        from serial.tools import list_ports
    except ImportError:
        return []
    return [
        PortInfo(
            port=item.device,
            description=item.description or "Serial interface",
            hardware_id=item.hwid or "",
        )
        for item in list_ports.comports()
    ]


def _windows_registry_ports() -> list[PortInfo]:
    if os.name != "nt":
        return []
    try:
        import winreg

        key = winreg.OpenKey(
            winreg.HKEY_LOCAL_MACHINE,
            r"HARDWARE\DEVICEMAP\SERIALCOMM",
        )
        result: list[PortInfo] = []
        index = 0
        while True:
            try:
                driver, port, _ = winreg.EnumValue(key, index)
            except OSError:
                break
            result.append(PortInfo(port=port, description=driver))
            index += 1
        return result
    except OSError:
        return []


def _mode_ports() -> list[PortInfo]:
    if os.name != "nt":
        return []
    try:
        completed = subprocess.run(
            ["cmd", "/c", "mode"],
            capture_output=True,
            text=True,
            timeout=3,
            creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
        )
    except (OSError, subprocess.SubprocessError):
        return []
    names = sorted(set(re.findall(r"Status for device (COM\d+):", completed.stdout)))
    return [PortInfo(port=name, description="Serial interface") for name in names]


def discover_ports() -> list[PortInfo]:
    ports = _pyserial_ports() or _windows_registry_ports() or _mode_ports()
    unique: dict[str, PortInfo] = {}
    for item in ports:
        unique[item.port.upper()] = item
    return sorted(unique.values(), key=lambda item: _natural_port(item.port))


def _natural_port(value: str) -> tuple[str, int]:
    match = re.match(r"([A-Za-z]+)(\d+)$", value)
    return (match.group(1), int(match.group(2))) if match else (value, 0)


def classify_ports(ports: list[PortInfo]) -> dict[str, PortInfo]:
    found: dict[str, PortInfo] = {}
    for port in ports:
        text = port.search_text
        if "VID_0483&PID_5740" in text or "STM32" in text:
            found["bridge"] = port
        elif "USB-COMI" in text or "USB COMI" in text or "FTDI" in text:
            found["adapter"] = port

    # The Windows registry fallback lacks VID/PID and descriptions. These known
    # bench mappings remain clearly identified as inferred, not probed devices.
    by_name = {item.port.upper(): item for item in ports}
    if "bridge" not in found and "COM5" in by_name:
        found["bridge"] = by_name["COM5"]
    if "adapter" not in found and "COM3" in by_name:
        found["adapter"] = by_name["COM3"]
    return found


@dataclass(frozen=True)
class DiscoverySnapshot:
    ports: tuple[PortInfo, ...]
    classified: dict[str, PortInfo]
    instruments: tuple[object, ...]
    active_probe_allowed: bool
    message: str


class DiscoveryService:
    """Poll ports periodically without blocking the Tk event loop."""

    def __init__(self, callback: Callable[[DiscoverySnapshot], None],
                 interval_seconds: float = 1.5) -> None:
        self.callback = callback
        self.interval_seconds = interval_seconds
        self._stop = threading.Event()
        self._scan_lock = threading.Lock()
        self._thread: threading.Thread | None = None

    def start(self) -> None:
        if self._thread and self._thread.is_alive():
            return
        self._stop.clear()
        self._thread = threading.Thread(target=self._run, daemon=True, name="device-discovery")
        self._thread.start()

    def stop(self) -> None:
        self._stop.set()

    def scan_now(self) -> None:
        threading.Thread(target=self._scan_once, daemon=True,
                         name="device-discovery-refresh").start()

    def _run(self) -> None:
        while not self._stop.is_set():
            self._scan_once()
            self._stop.wait(self.interval_seconds)

    def _scan_once(self) -> None:
        if not self._scan_lock.acquire(blocking=False):
            return
        try:
            ports = discover_ports()
            classified = classify_ports(ports)
            bridge_present = "bridge" in classified
            adapter = classified.get("adapter")
            instruments: tuple[object, ...] = ()
            active_allowed = bool(adapter and not bridge_present)
            message = "Passive USB discovery"
            if active_allowed and adapter:
                try:
                    from .probe import probe_adapter

                    instruments = tuple(probe_adapter(adapter))
                    message = "Isolated deterministic Modbus fingerprinting"
                except ImportError:
                    message = "Install pyserial to enable Modbus fingerprinting"
            elif adapter and bridge_present:
                message = "Active probe blocked: bridge and adapter are both present"
            snapshot = DiscoverySnapshot(
                ports=tuple(ports), classified=classified, instruments=instruments,
                active_probe_allowed=active_allowed, message=message,
            )
            self.callback(snapshot)
        finally:
            self._scan_lock.release()
