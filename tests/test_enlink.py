import unittest

from app.enlink import default_console_password, normalize_dev_eui, parse_enlink_banner


IAQ_BANNER = """
\x1b[H\x1b[2J
Synetica - enLink :: Wireless Sensor Networks
Region:        North American band (Hybrid FSB#1) on 915MHz
Firmware Code: FW-AQ-VCP+
Firmware Ver:  5.06
Description:   enLink Air Quality - Environmental Sensors
DevEui:        00-04-a3-0b-00-08-4f-86
Password:
"""


class EnlinkBannerTests(unittest.TestCase):
    def test_exact_iaq_banner_signature(self) -> None:
        result = parse_enlink_banner(IAQ_BANNER, "COM5")
        self.assertIsNotNone(result)
        assert result is not None
        self.assertEqual("iaq_plus", result.key)
        self.assertEqual("FW-AQ-VCP+", result.firmware_code)
        self.assertEqual("5.06", result.firmware)
        self.assertIn("915MHz", result.region)
        self.assertEqual("00-04-a3-0b-00-08-4f-86", result.dev_eui)
        self.assertEqual("4f86", result.derived_login)

    def test_login_is_last_four_normalized_eui_characters(self) -> None:
        self.assertEqual("0004a30b00084f86", normalize_dev_eui("00-04-A3-0B-00-08-4F-86"))
        self.assertEqual("4f86", default_console_password("0004a30b00084f86"))

    def test_invalid_eui_cannot_produce_login(self) -> None:
        with self.assertRaises(ValueError):
            default_console_password("84f86")

    def test_unrelated_serial_text_is_rejected(self) -> None:
        self.assertIsNone(parse_enlink_banner("ordinary serial device", "COM7"))

    def test_unknown_enlink_product_is_not_guessed(self) -> None:
        banner = IAQ_BANNER.replace("FW-AQ-VCP+", "FW-UNKNOWN").replace(
            "enLink Air Quality - Environmental Sensors", "Unrecognized Device")
        self.assertIsNone(parse_enlink_banner(banner, "COM5"))


if __name__ == "__main__":
    unittest.main()
