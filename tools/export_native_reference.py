"""Generate the native UI's reference data directly from the Python application.

Development-only: the Rust executable embeds the result and never launches Python.
Use --check in parity verification to detect drift from the application baseline.
"""
from __future__ import annotations
import argparse
import dataclasses
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from app.catalog import DEVICES, PREMADE_CONFIGS
from app.registers import REGISTER_MAPS
from app.regions import DEFAULT_REGION, RADIO_REGIONS

DESTINATION = ROOT / "native/modbus-configurator/assets/python-reference.json"
DECODE_CASES = DESTINATION.with_name("python-register-cases.json")

def render() -> str:
    devices = {}
    for key, device in DEVICES.items():
        item = dataclasses.asdict(device)
        # Python's readings and device facts are staged bench snapshots, not
        # runtime observations. Embed labels/units only, never those values.
        item.pop("facts", None)
        item["readouts"] = [{"label": r.label, "unit": r.unit} for r in device.readouts]
        item["artifact"] = device.artifact.relative_to(ROOT).as_posix() if device.artifact else None
        # Use the manufacturer model in the native configurator, not the old product branding.
        if key == "bridge":
            item["name"] = "Synetica ENL-MOD-32"
        devices[key] = item
    document = {
        "schema_version": 1,
        "source": ["app/catalog.py", "app/registers.py", "app/regions.py"],
        "devices": devices,
        "registers": {key: [dataclasses.asdict(r) for r in values] for key, values in REGISTER_MAPS.items()},
        "premade": {key: path.relative_to(ROOT).as_posix() for key, path in PREMADE_CONFIGS.items()},
        "regions": {key: dataclasses.asdict(region) for key, region in RADIO_REGIONS.items()},
        "default_region": DEFAULT_REGION,
    }
    from tools.native_reference_extensions import extend
    extend(document, ROOT)
    return json.dumps(document, ensure_ascii=False, indent=2) + "\n"

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    content = render()
    cases = [{"device": key, "index": index, "value": value, "expected": register.decode(value)}
             for key, registers in REGISTER_MAPS.items()
             for index, register in enumerate(registers)
             for value in (None, 0, 1, 5, 530, 65535, -1, 0x80000000)]
    outputs = {DESTINATION: content, DECODE_CASES: json.dumps(cases, ensure_ascii=False, indent=2) + "\n"}
    if args.check:
        for path, expected in outputs.items():
            if not path.exists() or path.read_text(encoding="utf-8") != expected:
                raise SystemExit("Native reference differs from Python. Run tools/export_native_reference.py.")
        print("Native reference matches Python catalog, registers, and regions.")
    else:
        for path, expected in outputs.items():
            path.write_text(expected, encoding="utf-8", newline="\n")
        print(f"Exported {len(DEVICES)} devices and {sum(map(len, REGISTER_MAPS.values()))} register definitions.")
