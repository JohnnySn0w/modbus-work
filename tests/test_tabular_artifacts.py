from __future__ import annotations

import csv
import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
TABULAR_DIRS = (ROOT / "artifacts/register-tables", ROOT / "artifacts/bridge-config", ROOT / ".secrets")
REGISTER_HEADERS = ["register number", "name", "units", "interpretation", "decode", "defaults"]
BRIDGE_HEADERS = ["Item", "ID", "Reg", "Addr", "Data", "Word", "Mult", "Read"]


class TabularArtifactTests(unittest.TestCase):
    def files(self, suffix: str) -> list[Path]:
        return sorted(path for directory in TABULAR_DIRS for path in directory.glob(f"*{suffix}"))

    def test_csv_and_tsv_files_are_plain_ascii_without_bom(self) -> None:
        for path in self.files(".csv") + self.files(".tsv"):
            with self.subTest(path=path.name):
                raw = path.read_bytes()
                self.assertFalse(raw.startswith(b"\xef\xbb\xbf"))
                raw.decode("ascii")

    def test_register_csv_schema_and_excel_safe_ranges(self) -> None:
        compact_numeric_range = re.compile(r"^\d+[-/]\d+$")
        compact_hex_range = re.compile(r"^0x[0-9A-Fa-f]+[-/]0x[0-9A-Fa-f]+$")
        for path in self.files(".csv"):
            with self.subTest(path=path.name):
                with path.open("r", encoding="ascii", newline="") as stream:
                    rows = list(csv.DictReader(stream))
                self.assertTrue(rows)
                self.assertEqual(list(rows[0]), REGISTER_HEADERS)
                for row in rows:
                    register = row["register number"]
                    self.assertIsNone(compact_numeric_range.fullmatch(register))
                    self.assertIsNone(compact_hex_range.fullmatch(register))
                    self.assertFalse(register.startswith(("=", "+", "-", "@")))
                    if " - " in register:
                        start, end = register.split(" - ", 1)
                        base = 16 if start.lower().startswith("0x") else 10
                        width = int(end, base) - int(start, base) + 1
                        self.assertIn(width, {1, 2, 4})

    def test_native_bridge_tsv_schema_and_supported_types(self) -> None:
        allowed_register_types = {"Hold", "Input"}
        allowed_data_types = {"U16", "S16", "U32", "S32", "F32"}
        for path in self.files(".tsv"):
            with self.subTest(path=path.name):
                with path.open("r", encoding="ascii", newline="") as stream:
                    rows = list(csv.DictReader(stream, delimiter="\t"))
                self.assertTrue(rows)
                self.assertEqual(list(rows[0]), BRIDGE_HEADERS)
                for row in rows:
                    self.assertIn(row["Reg"], allowed_register_types)
                    self.assertIn(row["Data"], allowed_data_types)


if __name__ == "__main__":
    unittest.main()
