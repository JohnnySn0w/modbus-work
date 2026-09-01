import unittest

from app.enlink import (
    default_console_password,
    normalize_dev_eui,
    parse_enlink_banner,
    parse_sensor_readings,
)


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

SENSOR_PAGE = """
Sensor Readings (Page 1):
     Temperature                        24.7?C
-- VOC Air Quality Sensor
     Temperature                        25.3?C
     Humidity                           54%
     Pressure                           997 mbar
     CO2e Estimate                      500 ppm
     bVOC Estimate                      0.50 ppm
     IAQ [Accuracy]                     25 IAQ [2:Medium]
-- CO2 Sensor (GSS)
     Version/Serial No                  LP26/614548
     Auto Calibration                   Enabled
     Reading                            88 ppm
"""

BRIDGE_BANNER = """
Synetica - enLink :: Wireless Sensor Networks
Region:        North American band (Hybrid) on 915MHz
Model Number:  ENL-MOD-32
Model Name:    enLink Modbus RS485 RTU Master
Firmware Ver:  3.6
DevEui:        00-04-a3-0b-00-05-cc-7c
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

    def test_bridge_banner_uses_model_fields_not_iaq_fields(self) -> None:
        result = parse_enlink_banner(BRIDGE_BANNER, "COM12")
        self.assertIsNotNone(result)
        assert result is not None
        self.assertEqual("bridge", result.key)
        self.assertEqual("ENL-MOD-32", result.firmware_code)
        self.assertEqual("3.6", result.firmware)
        self.assertEqual("cc7c", result.derived_login)

    def test_invalid_eui_cannot_produce_login(self) -> None:
        with self.assertRaises(ValueError):
            default_console_password("84f86")

    def test_unrelated_serial_text_is_rejected(self) -> None:
        self.assertIsNone(parse_enlink_banner("ordinary serial device", "COM7"))

    def test_unknown_enlink_product_is_not_guessed(self) -> None:
        banner = IAQ_BANNER.replace("FW-AQ-VCP+", "FW-UNKNOWN").replace(
            "enLink Air Quality - Environmental Sensors", "Unrecognized Device")
        self.assertIsNone(parse_enlink_banner(banner, "COM5"))

    def test_live_sensor_page_is_parsed_by_channel_context(self) -> None:
        readings = parse_sensor_readings(SENSOR_PAGE)
        self.assertEqual((24.7, "°C"), readings["Temperature"])
        self.assertEqual((54.0, "%RH"), readings["Relative humidity"])
        self.assertEqual((997.0, "mbar"), readings["Pressure"])
        self.assertEqual((500.0, "ppm"), readings["CO₂ equivalent"])
        self.assertEqual((0.5, "ppm"), readings["bVOC estimate"])
        self.assertEqual((25, "2:Medium"), readings["IAQ"])
        self.assertEqual((88.0, "ppm"), readings["CO₂"])

    def test_non_sensor_page_has_no_live_values(self) -> None:
        self.assertEqual({}, parse_sensor_readings(IAQ_BANNER))


if __name__ == "__main__":
    unittest.main()
