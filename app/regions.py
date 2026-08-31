from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class RadioRegion:
    key: str
    display_name: str
    frequency_mhz: str
    enabled: bool
    notes: str


DEFAULT_REGION = "us915_hybrid_fsb1"

RADIO_REGIONS = {
    "us915_hybrid_fsb1": RadioRegion(
        key="us915_hybrid_fsb1",
        display_name="North American Hybrid FSB #1",
        frequency_mhz="902–928",
        enabled=True,
        notes="Default for this system and all currently assumed hardware.",
    ),
    "eu868": RadioRegion(
        key="eu868",
        display_name="Europe EU868",
        frequency_mhz="863–870",
        enabled=False,
        notes="Prepared future option; enable only with matching regional hardware/firmware and Loriot network profile.",
    ),
}
