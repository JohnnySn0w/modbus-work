from __future__ import annotations

import struct
import time
from dataclasses import dataclass, field

from .discovery import PortInfo


@dataclass(frozen=True)
class ProbeResult:
    key: str
    port: str
    slave_id: int
    serial_format: str
    confidence: str
    reason: str
    readings: dict[str, tuple[float | int | str, str]] = field(default_factory=dict)


def modbus_crc(data: bytes) -> int:
    crc = 0xFFFF
    for value in data:
        crc ^= value
        for _ in range(8):
            crc = (crc >> 1) ^ 0xA001 if crc & 1 else crc >> 1
    return crc


def frame_with_crc(payload: bytes) -> bytes:
    return payload + modbus_crc(payload).to_bytes(2, "little")


def validate_response(response: bytes, slave_id: int, function: int) -> bytes | None:
    if len(response) < 5 or response[0] != slave_id:
        return None
    if modbus_crc(response[:-2]) != int.from_bytes(response[-2:], "little"):
        return None
    if response[1] == function | 0x80:
        return None
    if response[1] != function:
        return None
    return response[2:-2]


def decode_float_words(data: bytes, low_word_first: bool) -> float:
    if len(data) != 4:
        raise ValueError("A float requires exactly four data bytes")
    ordered = data[2:4] + data[0:2] if low_word_first else data
    return struct.unpack(">f", ordered)[0]


def decode_uint32_low_word_first(data: bytes) -> int:
    if len(data) != 4:
        raise ValueError("A 32-bit integer requires exactly four data bytes")
    low_word = int.from_bytes(data[0:2], "big")
    high_word = int.from_bytes(data[2:4], "big")
    return (high_word << 16) | low_word


class ModbusClient:
    def __init__(self, port: str, baud: int, parity: str, stopbits: int,
                 timeout: float = 0.14) -> None:
        self.port = port
        self.baud = baud
        self.parity = parity
        self.stopbits = stopbits
        self.timeout = timeout

    def _open(self):
        import serial

        return serial.Serial(
            self.port,
            self.baud,
            bytesize=8,
            parity=self.parity,
            stopbits=self.stopbits,
            timeout=self.timeout,
            write_timeout=self.timeout,
        )

    def request(self, slave: int, function: int, address: int, count: int) -> bytes | None:
        payload = bytes((slave, function)) + address.to_bytes(2, "big") + count.to_bytes(2, "big")
        query = frame_with_crc(payload)
        try:
            with self._open() as connection:
                connection.reset_input_buffer()
                connection.write(query)
                connection.flush()
                expected = 5 + count * 2
                response = connection.read(expected)
        except (OSError, ValueError):
            return None
        body = validate_response(response, slave, function)
        if not body or body[0] != count * 2 or len(body[1:]) != count * 2:
            return None
        return body[1:]

    def report_slave_id(self, slave: int) -> bytes | None:
        query = frame_with_crc(bytes((slave, 0x11)))
        try:
            with self._open() as connection:
                connection.reset_input_buffer()
                connection.write(query)
                connection.flush()
                response = connection.read(128)
        except (OSError, ValueError):
            return None
        return validate_response(response, slave, 0x11)


def probe_adapter(port: PortInfo) -> list[ProbeResult]:
    """Run bounded, read-only fingerprints on an isolated RS-485 adapter.

    The candidate list is intentionally small. It does not sweep all 247 slave
    addresses or change serial settings on any instrument.
    """
    probes = (
        _probe_dpt146(port.port, 1, "N", 2),
        _probe_hmd65(port.port, 1, "N", 1),
        _probe_wattnode(port.port, 1, "N", 1),
        _probe_wattnode(port.port, 127, "N", 1),
        _probe_dpt146(port.port, 240, "E", 1),
    )
    results: list[ProbeResult] = []
    for probe in probes:
        result = probe()
        if result:
            results.append(result)
            if result.confidence == "high":
                break
    return results


def _probe_dpt146(port: str, slave: int, parity: str, stopbits: int):
    def run() -> ProbeResult | None:
        client = ModbusClient(port, 19200, parity, stopbits)
        pressure_raw = client.request(slave, 3, 44, 2)
        moisture_raw = client.request(slave, 3, 20, 2)
        status_raw = client.request(slave, 3, 512, 2)
        if not pressure_raw or not moisture_raw or not status_raw:
            return None
        try:
            pressure = decode_float_words(pressure_raw, low_word_first=True)
            moisture = decode_float_words(moisture_raw, low_word_first=True)
        except (ValueError, struct.error):
            return None
        fault, online = struct.unpack(">HH", status_raw)
        if not (0.0 <= pressure <= 12.0 and 0.0 <= moisture <= 1_000_000 and fault in (0, 1)):
            return None
        return ProbeResult(
            key="dpt146", port=port, slave_id=slave,
            serial_format=f"19200 8{parity}{stopbits}", confidence="high",
            reason="DPT146 pressure, moisture, and status fingerprint matched.",
            readings={
                "Absolute pressure": (round(pressure, 4), "bara"),
                "Moisture": (round(moisture, 1), "ppmv"),
                "Fault status": (fault, ""),
                "Online status": (online, ""),
            },
        )
    return run


def _probe_hmd65(port: str, slave: int, parity: str, stopbits: int):
    def run() -> ProbeResult | None:
        client = ModbusClient(port, 19200, parity, stopbits)
        values = client.request(slave, 3, 0, 16)
        status = client.request(slave, 3, 512, 7)
        if not values or not status:
            return None
        candidates = []
        for low_first in (False, True):
            try:
                decoded = [decode_float_words(values[index:index + 4], low_first)
                           for index in range(0, 32, 4)]
            except (ValueError, struct.error):
                continue
            humidity, temperature = decoded[0], decoded[1]
            temperature_like = decoded[2], decoded[3], decoded[6]
            if (0 <= humidity <= 100 and -80 <= temperature <= 120
                    and all(-120 <= value <= 180 for value in temperature_like)
                    and decoded[4] >= 0 and decoded[5] >= 0):
                candidates.append((decoded, low_first))
        if len(candidates) != 1:
            return None
        decoded, low_first = candidates[0]
        humidity, temperature = decoded[0], decoded[1]
        device_status = int.from_bytes(status[:2], "big", signed=True)
        rh_status = int.from_bytes(status[10:12], "big", signed=True)
        temperature_status = int.from_bytes(status[12:14], "big", signed=True)
        if any(value < 0 or value > 0x1FF for value in (device_status, rh_status, temperature_status)):
            return None
        return ProbeResult(
            key="hmd65", port=port, slave_id=slave,
            serial_format=f"19200 8{parity}{stopbits}", confidence="high",
            reason="All eight HMD65 measurement floats and its status layout matched.",
            readings={
                "Relative humidity": (round(humidity, 2), "%RH"),
                "Temperature": (round(temperature, 2), "°C"),
                "Device status": (device_status, ""),
                "Float order": ("HL" if low_first else "HH", ""),
            },
        )
    return run


def _probe_wattnode(port: str, slave: int, parity: str, stopbits: int):
    def run() -> ProbeResult | None:
        client = ModbusClient(port, 19200, parity, stopbits)
        identity = client.report_slave_id(slave)
        if not identity:
            return None
        text = identity.decode("ascii", errors="ignore")
        if "WattNode" not in text and "Continental Control Systems" not in text:
            return None
        # Manual registers are one-based; request() takes the zero-based PDU
        # address. Model=530 identifies the WND meter-module family. The
        # Report Slave ID string does not identify the exact M1/M0 enclosure.
        diagnostics = client.request(slave, 3, 1700, 8)
        if not diagnostics or len(diagnostics) != 16:
            return None
        serial_number = decode_uint32_low_word_first(diagnostics[0:4])
        model = int.from_bytes(diagnostics[12:14], "big")
        firmware = int.from_bytes(diagnostics[14:16], "big")
        if model != 530 or not (1000 <= firmware < 1100) or serial_number <= 0:
            return None
        return ProbeResult(
            key="wattnode", port=port, slave_id=slave,
            serial_format=f"19200 8{parity}{stopbits}",
            confidence="family",
            reason=("Report Slave ID and model register 530 identified a WND meter module; "
                    "confirm WND-M1-MB on the physical label."),
            readings={
                "Identity": (text.strip("\x00"), ""),
                "Serial number": (serial_number, ""),
                "Model code": (model, ""),
                "Firmware": (firmware, ""),
            },
        )
    return run
