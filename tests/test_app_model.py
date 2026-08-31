import unittest

from app.catalog import DEVICES, PREMADE_CONFIGS
from app.discovery import PortInfo, classify_ports
from app.regions import DEFAULT_REGION, RADIO_REGIONS


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
        for path in PREMADE_CONFIGS.values():
            self.assertTrue(path.is_file(), path)

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


if __name__ == "__main__":
    unittest.main()
