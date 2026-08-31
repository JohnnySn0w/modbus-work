from __future__ import annotations

import argparse
import json
import sys
from datetime import datetime
from dataclasses import asdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from app.enlink import capture_read_only_snapshot


def main() -> None:
    parser = argparse.ArgumentParser(description="Capture read-only Synetica enLink console menus")
    parser.add_argument("--port", default="COM5")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    payload = {
        "schema_version": 1,
        "captured_at": datetime.now().astimezone().isoformat(),
        **asdict(capture_read_only_snapshot(args.port)),
    }
    text = json.dumps(payload, indent=2, ensure_ascii=False)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(text + "\n", encoding="utf-8")
        print(f"Saved read-only snapshot to {args.output}")
    else:
        print(text)


if __name__ == "__main__":
    main()
