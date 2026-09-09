from __future__ import annotations

import json
import re
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path


TABLE_HEADER = ("Item", "ID", "Reg", "Addr", "Data", "Word", "Mult", "Read")
ROW_PATTERN = re.compile(r"^\d+\t\d+\t(?:Hold|Input)\t\d+\t\S+\t\S+\t\S+\t\S+$")


@dataclass(frozen=True)
class BridgeApplyResult:
    backup_rows: tuple[str, ...]
    applied_rows: tuple[str, ...]
    readback_rows: tuple[str, ...]
    acknowledgements: tuple[str, ...]
    read_summary: str


class BridgeApplyError(RuntimeError):
    def __init__(self, message: str, backup_rows: tuple[str, ...], rollback_ok: bool) -> None:
        super().__init__(message)
        self.backup_rows = backup_rows
        self.rollback_ok = rollback_ok


READ_VALUE_PATTERN = re.compile(
    r"(?im)^\s*(?:item\s*)?(\d{1,2})\s*(?:[:=\t]|\s{2,})\s*"
    r"([-+]?(?:\d+(?:\.\d*)?|\.\d+)(?:e[-+]?\d+)?)\s*$"
)


def parse_read_values(text: str, expected_count: int) -> tuple[float | int, ...]:
    values: dict[int, float | int] = {}
    for match in READ_VALUE_PATTERN.finditer(text.replace("\r", "")):
        item = int(match.group(1))
        raw = float(match.group(2))
        values[item] = int(raw) if raw.is_integer() else raw
    expected = set(range(1, expected_count + 1))
    if set(values) != expected:
        raise RuntimeError(
            f"Bridge returned values for items {sorted(values)}; expected 1 through {expected_count}"
        )
    return tuple(values[item] for item in range(1, expected_count + 1))


def load_bridge_table(path: Path) -> tuple[str, ...]:
    lines = [line.rstrip("\r\n") for line in path.read_text(encoding="utf-8-sig").splitlines()]
    if not lines or tuple(lines[0].split("\t")) != TABLE_HEADER:
        raise ValueError("Configuration must start with the eight-column ENL-MOD-32 TSV header")
    rows = tuple(line for line in lines[1:] if line.strip())
    if not rows or len(rows) > 32:
        raise ValueError("Configuration must contain between 1 and 32 data rows")
    for expected_item, row in enumerate(rows, 1):
        columns = row.split("\t")
        if len(columns) != 8 or not ROW_PATTERN.fullmatch(row):
            raise ValueError(f"Invalid bridge configuration row {expected_item}: {row}")
        if int(columns[0]) != expected_item:
            raise ValueError("Configuration item numbers must be contiguous and start at 1")
        if not 0 <= int(columns[1]) <= 247:
            raise ValueError(f"Invalid Modbus slave ID in item {expected_item}")
    return rows


def write_bridge_table(path: Path, rows: tuple[str, ...]) -> None:
    path.write_text("\t".join(TABLE_HEADER) + "\n" + "\n".join(rows) + "\n", encoding="utf-8")


def parse_export_rows(text: str) -> tuple[str, ...]:
    clean = re.sub(r"\x1b\[[0-?]*[ -/]*[@-~]", "", text).replace("\r", "")
    rows = tuple(line.strip() for line in clean.splitlines() if ROW_PATTERN.fullmatch(line.strip()))
    return rows


class EnlinkModbusBridge:
    """Firmware 3.6 adapter backed by the validated Windows/.NET transport."""

    def __init__(self, port: str) -> None:
        self.port = port

    def _run_transport(self, action: str, config_path: Path | None = None,
                       backup_path: Path | None = None) -> dict[str, object]:
        root = Path(__file__).resolve().parents[1]
        script = root / "tools" / "enlink_bridge_config.ps1"
        command = [
            "powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", str(script),
            "-PortName", self.port, "-Action", action,
        ]
        if config_path is not None:
            command.extend(("-ConfigPath", str(config_path)))
        if backup_path is not None:
            command.extend(("-BackupPath", str(backup_path)))
        completed = subprocess.run(
            command, capture_output=True, text=True, timeout=180,
            creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
        )
        if completed.returncode:
            message = completed.stderr.strip() or completed.stdout.strip() or "Bridge console operation failed"
            raise RuntimeError(message)
        try:
            return json.loads(completed.stdout)
        except json.JSONDecodeError as exc:
            raise RuntimeError("Bridge console returned invalid completion data") from exc

    def export_verified(self) -> tuple[str, ...]:
        payload = self._run_transport("Export")
        rows = tuple(payload["backupRows"])  # type: ignore[arg-type]
        if not rows:
            raise RuntimeError("Bridge export returned no configured point rows")
        return rows

    def read_all_verified(self) -> tuple[tuple[float | int, ...], str]:
        payload = self._run_transport("Read")
        rows = tuple(payload["backupRows"])  # type: ignore[arg-type]
        if not rows:
            raise RuntimeError("Bridge has no configured point rows")
        summary = str(payload["readSummary"])
        expected_summary = f"{len(rows)}/0 (OK/Exceptions)"
        if summary != expected_summary:
            raise RuntimeError(f"Read All returned {summary}; expected {expected_summary}")
        return parse_read_values(str(payload["readText"]), len(rows)), summary

    def apply_verified(self, rows: tuple[str, ...]) -> BridgeApplyResult:
        with tempfile.TemporaryDirectory(prefix="modbus-bridge-") as folder:
            config_path = Path(folder) / "selected.tsv"
            backup_path = Path(folder) / "backup.tsv"
            write_bridge_table(config_path, rows)
            try:
                payload = self._run_transport("Apply", config_path, backup_path)
            except RuntimeError as exc:
                backup = load_bridge_table(backup_path) if backup_path.exists() else ()
                raise BridgeApplyError(str(exc), backup, False) from exc
            return BridgeApplyResult(
                backup_rows=tuple(payload["backupRows"]),
                applied_rows=tuple(payload["appliedRows"]),
                readback_rows=tuple(payload["readbackRows"]),
                acknowledgements=tuple(payload["acknowledgements"]),
                read_summary=str(payload["readSummary"]),
            )
