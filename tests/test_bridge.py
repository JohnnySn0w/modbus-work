from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from app.bridge import TABLE_HEADER, load_bridge_table, parse_export_rows, write_bridge_table


class BridgeConfigurationTests(unittest.TestCase):
    def test_validated_dpt_table_loads(self) -> None:
        rows = load_bridge_table(Path("artifacts/bridge-config/vaisala-dpt146-golden.tsv"))
        self.assertEqual(8, len(rows))
        self.assertEqual("1", rows[0].split("\t")[1])

    def test_export_parser_ignores_console_text(self) -> None:
        text = "menu\r\n" + "\t".join(TABLE_HEADER) + "\r\n1\t1\tHold\t4\tF32\tHL\t1\tInt\r\nPress a key"
        self.assertEqual(("1\t1\tHold\t4\tF32\tHL\t1\tInt",), parse_export_rows(text))

    def test_round_trip_backup_table(self) -> None:
        rows = ("1\t1\tHold\t4\tF32\tHL\t1\tInt",)
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "backup.tsv"
            write_bridge_table(path, rows)
            self.assertEqual(rows, load_bridge_table(path))

    def test_rejects_noncontiguous_items(self) -> None:
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "bad.tsv"
            path.write_text("\t".join(TABLE_HEADER) + "\n2\t1\tHold\t4\tF32\tHL\t1\tInt\n")
            with self.assertRaises(ValueError):
                load_bridge_table(path)


if __name__ == "__main__":
    unittest.main()
