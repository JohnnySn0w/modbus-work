from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class FirmwareValidation:
    valid: bool
    errors: tuple[str, ...]
    warnings: tuple[str, ...]
    image_path: Path | None = None
    target_version: str | None = None


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def validate_firmware_manifest(
    manifest_path: Path,
    *,
    product_family: str,
    firmware_code: str,
    current_version: str,
    region_key: str,
) -> FirmwareValidation:
    errors: list[str] = []
    warnings: list[str] = []
    try:
        document = json.loads(manifest_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        return FirmwareValidation(False, (f"Cannot read manifest: {exc}",), ())

    required = ("schema_version", "product_family", "firmware_code", "target_version",
                "regions", "image", "image_sha256", "flash_method")
    missing = [name for name in required if name not in document]
    if missing:
        return FirmwareValidation(False, ("Missing fields: " + ", ".join(missing),), ())

    if document["schema_version"] != 1:
        errors.append("Unsupported firmware manifest schema")
    if document["product_family"] != product_family:
        errors.append("Package product family does not match the connected device")
    if document["firmware_code"] != firmware_code:
        errors.append("Package firmware code does not match the connected device")
    if region_key not in document["regions"]:
        errors.append("Package is not approved for the connected radio region")

    image_path = (manifest_path.parent / str(document["image"])).resolve()
    package_root = manifest_path.parent.resolve()
    if package_root not in image_path.parents:
        errors.append("Firmware image must remain inside its package directory")
    elif not image_path.is_file():
        errors.append("Firmware image is missing")
    else:
        expected = str(document["image_sha256"]).lower()
        actual = sha256_file(image_path)
        if expected != actual:
            errors.append("Firmware image SHA-256 does not match the manifest record")

    allowed_from = document.get("allowed_from_versions", [])
    if allowed_from and current_version not in allowed_from:
        errors.append("Current firmware is not in this package's allowed upgrade paths")
    if str(document["target_version"]) == current_version:
        warnings.append("Target version is already installed")
    if not document.get("vendor_approved", False):
        errors.append("Package is not marked vendor-approved")
    if not document.get("recovery_procedure"):
        errors.append("Package does not identify a recovery procedure")

    return FirmwareValidation(
        valid=not errors,
        errors=tuple(errors),
        warnings=tuple(warnings),
        image_path=image_path if image_path.is_file() else None,
        target_version=str(document["target_version"]),
    )
