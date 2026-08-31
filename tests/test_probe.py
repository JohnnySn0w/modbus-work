import math
import struct
import unittest

from app.probe import decode_float_words, frame_with_crc, modbus_crc, validate_response


class ProbeProtocolTests(unittest.TestCase):
    def test_known_modbus_request_crc(self) -> None:
        payload = bytes.fromhex("01 03 00 04 00 02")
        self.assertEqual(frame_with_crc(payload).hex(" "), "01 03 00 04 00 02 85 ca")

    def test_validates_captured_dpt146_response(self) -> None:
        frame = bytes.fromhex("01 03 04 83 28 41 da e2 74")
        body = validate_response(frame, slave_id=1, function=3)
        self.assertEqual(body, bytes.fromhex("04 83 28 41 da"))
        value = decode_float_words(body[1:], low_word_first=True)
        self.assertTrue(math.isclose(value, 27.314, rel_tol=0, abs_tol=0.001))

    def test_rejects_bad_crc_and_wrong_function(self) -> None:
        frame = bytearray.fromhex("01 03 04 83 28 41 da e2 74")
        frame[-1] ^= 0x01
        self.assertIsNone(validate_response(bytes(frame), 1, 3))
        valid = frame_with_crc(bytes.fromhex("01 04 02 00 00"))
        self.assertIsNone(validate_response(valid, 1, 3))

    def test_decodes_both_float_word_orders(self) -> None:
        high_first = struct.pack(">f", 42.5)
        low_first = high_first[2:] + high_first[:2]
        self.assertEqual(decode_float_words(high_first, False), 42.5)
        self.assertEqual(decode_float_words(low_first, True), 42.5)

    def test_crc_function_matches_appended_crc(self) -> None:
        payload = bytes.fromhex("f0 03 00 14 00 02")
        frame = frame_with_crc(payload)
        self.assertEqual(modbus_crc(frame[:-2]), int.from_bytes(frame[-2:], "little"))


if __name__ == "__main__":
    unittest.main()
