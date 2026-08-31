import unittest

from app.catalog import DEVICES
from app.discovery import PortInfo, classify_ports


class CatalogTests(unittest.TestCase):
    def test_active_device_scope_is_present(self) -> None:
        self.assertEqual({"dpt146", "hmd65", "wattnode"},
                         {key for key in DEVICES if key in {"dpt146", "hmd65", "wattnode"}})
        self.assertEqual("Golden configuration", DEVICES["dpt146"].status)
        self.assertEqual("Ready to test", DEVICES["hmd65"].status)
        self.assertEqual("Ready to test", DEVICES["wattnode"].status)

    def test_bridge_vid_pid_detection(self) -> None:
        ports = [PortInfo("COM8", "USB Serial Device", "USB VID_0483&PID_5740")]
        self.assertEqual("COM8", classify_ports(ports)["bridge"].port)

    def test_known_bench_port_fallbacks(self) -> None:
        ports = [
            PortInfo("COM3", "Serial interface"),
            PortInfo("COM5", "Serial interface"),
        ]
        found = classify_ports(ports)
        self.assertEqual("COM3", found["adapter"].port)
        self.assertEqual("COM5", found["bridge"].port)


if __name__ == "__main__":
    unittest.main()
