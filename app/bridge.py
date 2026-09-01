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

    def apply_verified(self, rows: tuple[str, ...]) -> BridgeApplyResult:
        root = Path(__file__).resolve().parents[1]
        script = root / "tools" / "enlink_bridge_config.ps1"
        with tempfile.TemporaryDirectory(prefix="modbus-bridge-") as folder:
            config_path = Path(folder) / "selected.tsv"
            backup_path = Path(folder) / "backup.tsv"
            write_bridge_table(config_path, rows)
            completed = subprocess.run(
                ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", str(script),
                 "-PortName", self.port, "-Action", "Apply", "-ConfigPath", str(config_path),
                 "-BackupPath", str(backup_path)],
                capture_output=True, text=True, timeout=180,
                creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
            )
            backup = load_bridge_table(backup_path) if backup_path.exists() else ()
            if completed.returncode:
                message = completed.stderr.strip() or completed.stdout.strip() or "Bridge writer failed"
                raise BridgeApplyError(message, backup, False)
            try:
                payload = json.loads(completed.stdout)
            except json.JSONDecodeError as exc:
                raise BridgeApplyError("Bridge writer returned invalid completion data", backup, False) from exc
            return BridgeApplyResult(
                backup_rows=tuple(payload["backupRows"]),
                applied_rows=tuple(payload["appliedRows"]),
                readback_rows=tuple(payload["readbackRows"]),
                acknowledgements=tuple(payload["acknowledgements"]),
                read_summary=str(payload["readSummary"]),
            )
