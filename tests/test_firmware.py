import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from app.firmware import validate_firmware_manifest


class FirmwarePackageTests(unittest.TestCase):
    def make_package(self, root: Path, **overrides) -> Path:
        image = root / "iaq-5.07.bin"
        image.write_bytes(b"vendor firmware fixture")
        document = {
            "schema_version": 1,
            "product_family": "Synetica enLink IAQ Plus",
            "firmware_code": "FW-AQ-VCP+",
            "target_version": "5.07",
            "allowed_from_versions": ["5.06"],
            "regions": ["us915_hybrid_fsb1"],
            "image": image.name,
            "image_sha256": hashlib.sha256(image.read_bytes()).hexdigest(),
            "flash_method": "vendor-updater-v1",
            "vendor_approved": True,
            "recovery_procedure": "vendor-recovery-v1",
        }
        document.update(overrides)
        manifest = root / "manifest.json"
        manifest.write_text(json.dumps(document), encoding="utf-8")
        return manifest

    def validate(self, manifest: Path):
        return validate_firmware_manifest(
            manifest,
            product_family="Synetica enLink IAQ Plus",
            firmware_code="FW-AQ-VCP+",
            current_version="5.06",
            region_key="us915_hybrid_fsb1",
        )

    def test_valid_vendor_package_passes_preflight(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            result = self.validate(self.make_package(Path(directory)))
            self.assertTrue(result.valid, result.errors)
            self.assertEqual("5.07", result.target_version)

    def test_hash_mismatch_is_blocked(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            result = self.validate(self.make_package(Path(directory), image_sha256="0" * 64))
            self.assertFalse(result.valid)
            self.assertTrue(any("SHA-256" in error for error in result.errors))

    def test_wrong_region_and_upgrade_path_are_blocked(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            manifest = self.make_package(Path(directory), regions=["eu868"], allowed_from_versions=["4.00"])
            result = self.validate(manifest)
            self.assertFalse(result.valid)
            self.assertTrue(any("region" in error for error in result.errors))
            self.assertTrue(any("upgrade paths" in error for error in result.errors))

    def test_unapproved_package_without_recovery_is_blocked(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            result = self.validate(self.make_package(
                Path(directory), vendor_approved=False, recovery_procedure=None,
            ))
            self.assertFalse(result.valid)
            self.assertTrue(any("vendor-approved" in error for error in result.errors))
            self.assertTrue(any("recovery" in error for error in result.errors))


if __name__ == "__main__":
    unittest.main()
