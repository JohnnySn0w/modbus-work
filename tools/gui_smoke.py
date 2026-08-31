from __future__ import annotations

import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from app.main import App


def pump(app: App, seconds: float) -> None:
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        app.update()
        time.sleep(0.05)


def main() -> None:
    app = App()
    try:
        pump(app, 6)
        if "iaq_plus" not in app.classified:
            raise RuntimeError(f"IAQ Plus not discovered: {sorted(app.classified)}")
        app.refresh_iaq_live()
        pump(app, 12)
        readings = app.live_readings.get("iaq_plus", {})
        if len(readings) < 7:
            raise RuntimeError(f"Expected at least seven live values, got {sorted(readings)}")
        print(f"GUI smoke passed: IAQ Plus detected with {len(readings)} live values")
    finally:
        app.close_app()


if __name__ == "__main__":
    main()
