import unittest
import csv

from app.catalog import DEVICES, PREMADE_CONFIGS
from app.discovery import PortInfo, classify_ports
from app.regions import DEFAULT_REGION, RADIO_REGIONS
from app.registers import REGISTER_MAPS


class CatalogTests(unittest.TestCase):
    def test_active_device_scope_is_present(self) -> None:
        self.assertTrue({"dpt146", "hmd65", "wattnode", "iaq_plus"}.issubset(DEVICES))
        self.assertEqual("Golden configuration", DEVICES["dpt146"].status)
        self.assertEqual("Ready to test", DEVICES["hmd65"].status)
        self.assertEqual("Ready to test", DEVICES["wattnode"].status)

    def test_bridge_vid_pid_detection(self) -> None:
        ports = [PortInfo("COM8", "USB Serial Device", "USB VID_0483&PID_5740")]
        self.assertEqual("COM8", classify_ports(ports)["synetica_usb"].port)

    def test_pyserial_stm32_vid_pid_format_detection(self) -> None:
        ports = [PortInfo("COM5", "USB Serial Device", "USB VID:PID=0483:5740 SER=ABC")]
        self.assertEqual("COM5", classify_ports(ports)["synetica_usb"].port)

    def test_premade_configurations_exist(self) -> None:
        self.assertEqual({"dpt146", "hmd65", "wattnode"}, set(PREMADE_CONFIGS))

    def test_wattnode_bridge_candidate_is_contiguous_and_low_word_first(self) -> None:
        path = PREMADE_CONFIGS["wattnode"]
        with path.open(newline="", encoding="utf-8") as handle:
            rows = list(csv.DictReader(handle, delimiter="\t"))
        self.assertEqual(list(range(1, 13)), [int(row["Item"]) for row in rows])
        self.assertTrue(all(row["Data"] == "F32" and row["Word"] == "HL" for row in rows))
        self.assertTrue(all(int(row["ID"]) == 1 for row in rows))
        addresses = [int(row["Addr"]) for row in rows]
        self.assertEqual(len(addresses), len(set(addresses)))
        for path in PREMADE_CONFIGS.values():
            self.assertTrue(path.is_file(), path)

    def test_each_modbus_device_has_a_register_table(self) -> None:
        self.assertEqual({"dpt146", "hmd65", "wattnode"}, set(REGISTER_MAPS))
        for key, registers in REGISTER_MAPS.items():
            self.assertGreater(len(registers), 0, key)
            self.assertTrue(all(item.logical and item.pdu and item.description for item in registers))

    def test_wattnode_register_table_uses_documented_addresses(self) -> None:
        by_name = {item.name: item for item in REGISTER_MAPS["wattnode"]}
        self.assertEqual(("1001–1002", "1000–1001"),
                         (by_name["Energy total"].logical, by_name["Energy total"].pdu))
        self.assertEqual(("1707", "1706"),
                         (by_name["Model"].logical, by_name["Model"].pdu))
        self.assertEqual("R/W", by_name["Connection type"].access)

    def test_every_device_has_basic_help(self) -> None:
        for device in DEVICES.values():
            self.assertGreaterEqual(len(device.help_setup), 3)
            self.assertGreaterEqual(len(device.help_troubleshooting), 3)

    def test_com_numbers_are_not_device_identities(self) -> None:
        ports = [
            PortInfo("COM3", "Serial interface"),
            PortInfo("COM5", "Serial interface"),
        ]
        found = classify_ports(ports)
        self.assertEqual({}, found)

    def test_radio_regions_default_to_us_with_eu_stub_disabled(self) -> None:
        self.assertEqual("us915_hybrid_fsb1", DEFAULT_REGION)
        self.assertTrue(RADIO_REGIONS[DEFAULT_REGION].enabled)
        self.assertFalse(RADIO_REGIONS["eu868"].enabled)

    def test_iaq_target_preserves_join_eui(self) -> None:
        target = (DEVICES["iaq_plus"].artifact.parents[1] / "native-config" /
                  "synetica-enlink-iaq-plus-us915.yaml")
        text = target.read_text(encoding="utf-8")
        self.assertIn("join_eui_policy: preserve-existing", text)
        self.assertIn("app_key_policy: provision-selected-credential-profile", text)
        self.assertNotIn("app_key_source: .secrets/synetica-enlink-iaq-plus.env", text)


if __name__ == "__main__":
    unittest.main()
