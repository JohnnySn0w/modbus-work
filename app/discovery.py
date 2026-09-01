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
            result.append(PortInfo(
                port=port,
                description=driver,
                hardware_id=_windows_usb_hardware_id(port),
            ))
            index += 1
        return result
    except OSError:
        return []


def _windows_usb_hardware_id(port: str) -> str:
    """Recover VID/PID for registry-only discovery without admin privileges."""
    try:
        completed = subprocess.run(
            ["reg", "query", r"HKLM\SYSTEM\CurrentControlSet\Enum\USB", "/s", "/f", port],
            capture_output=True,
            text=True,
            timeout=3,
            creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
        )
    except (OSError, subprocess.SubprocessError):
        return ""
    match = re.search(r"USB\\(VID_[0-9A-F]{4}&PID_[0-9A-F]{4})\\([^\r\n\\]+)",
                      completed.stdout, re.IGNORECASE)
    return "\\".join(match.groups()) if match else ""


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
        if ("VID_0483&PID_5740" in text or "VID:PID=0483:5740" in text
                or "STM32" in text):
            # Synetica uses this STM32 virtual-COM identity across products.
            # The USB console banner, not VID/PID or COM number, identifies it.
            found["synetica_usb"] = port
        elif ("USB-COMI" in text or "USB COMI" in text or "FTDI" in text
              or ("VID:PID=0403:6001" in text and "A7TLR1HQA" in text)
              or ("VID_0403&PID_6001" in text and "A7TLR1HQA" in text)):
            # The observed USB-COMi-TB exposes a generic FTDI description under
            # Windows. Bind its recorded USB serial as well as named drivers;
            # VID/PID 0403:6001 alone is shared by many unrelated adapters.
            found["adapter"] = port

    return found


def active_modbus_probe_allowed(classified: dict[str, PortInfo]) -> bool:
    """Allow reads only with an adapter and no known/potential bridge master."""
    return bool("adapter" in classified
                and "bridge" not in classified
                and "synetica_usb" not in classified)


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
        self._console_cache: dict[tuple[str, str], object] = {}

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
            instruments_list: list[object] = []
            console_port = classified.pop("synetica_usb", None)
            console_message = ""
            if console_port:
                cache_key = (console_port.port, console_port.hardware_id)
                console_result = self._console_cache.get(cache_key)
                try:
                    from .enlink import probe_enlink_console

                    if console_result is None:
                        console_result = probe_enlink_console(console_port)
                except ImportError:
                    console_result = None
                if console_result:
                    self._console_cache = {cache_key: console_result}
                    classified[console_result.key] = console_port
                    instruments_list.append(console_result)
                    console_message = f"Identified {console_result.display_name} by USB banner"
                else:
                    classified["synetica_usb"] = console_port
            # A shared Synetica VID/PID cannot identify the product, but it is
            # enough to prove that another potential Modbus master is present.
            # Keep active RS-485 probing disabled until the USB banner proves
            # whether the unit is a bridge or a sensor.
            bridge_present = "bridge" in classified or "synetica_usb" in classified
            adapter = classified.get("adapter")
            active_allowed = active_modbus_probe_allowed(classified)
            message = console_message or "Passive USB discovery"
            if active_allowed and adapter:
                try:
                    from .probe import probe_adapter

                    instruments_list.extend(probe_adapter(adapter))
                    message = "Isolated deterministic Modbus fingerprinting"
                except ImportError:
                    message = "Install pyserial to enable Modbus fingerprinting"
            elif adapter and bridge_present:
                message = "Active probe blocked: bridge and adapter are both present"
            snapshot = DiscoverySnapshot(
                ports=tuple(ports), classified=classified, instruments=tuple(instruments_list),
                active_probe_allowed=active_allowed, message=message,
            )
            self.callback(snapshot)
        finally:
            self._scan_lock.release()
