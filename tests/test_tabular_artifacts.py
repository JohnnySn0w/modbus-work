from __future__ import annotations

import csv
import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
TABULAR_DIRS = (ROOT / "artifacts/register-tables", ROOT / "artifacts/bridge-config", ROOT / ".secrets")
REGISTER_HEADERS = ["register number", "name", "units", "interpretation", "decode", "defaults"]
BRIDGE_HEADERS = ["Item", "ID", "Reg", "Addr", "Data", "Word", "Mult", "Read"]
EXPECTED_TEST_PROFILES = {
    "ati-badger-f12-d12-documentation-test.tsv",
    "hmd65-documentation-test.tsv",
    "micronics-u1000mkii-hm-documentation-test.tsv",
    "micronics-u3000-uf3300-documentation-test.tsv",
    "precision-digital-pd2-6000-documentation-test.tsv",
    "rki-voc-pro-documentation-test.tsv",
    "seeed-sensecap-s200-documentation-test.tsv",
    "wattnode-wnd-m1-mb-documentation-test.tsv",
}


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
        allowed_word_orders = {"HH", "HL", "LH", "LL"}
        for path in self.files(".tsv"):
            with self.subTest(path=path.name):
                with path.open("r", encoding="ascii", newline="") as stream:
                    rows = list(csv.DictReader(stream, delimiter="\t"))
                self.assertTrue(rows)
                self.assertEqual(list(rows[0]), BRIDGE_HEADERS)
                for row in rows:
                    self.assertIn(row["Reg"], allowed_register_types)
                    self.assertIn(row["Data"], allowed_data_types)
                    self.assertIn(row["Word"], allowed_word_orders)

    def test_documented_devices_have_bridge_test_profiles(self) -> None:
        actual = {path.name for path in (ROOT / "artifacts/bridge-config").glob("*-documentation-test.tsv")}
        self.assertEqual(actual, EXPECTED_TEST_PROFILES)

    def test_published_decodes_are_not_left_as_firmware_placeholders(self) -> None:
        forbidden = ("defined by firmware", "model-dependent sensor enumeration", "firmware enumeration")
        for path in self.files(".csv"):
            text = path.read_text(encoding="ascii").lower()
            with self.subTest(path=path.name):
                for phrase in forbidden:
                    self.assertNotIn(phrase, text)


if __name__ == "__main__":
    unittest.main()
