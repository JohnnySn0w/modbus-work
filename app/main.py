from __future__ import annotations

import os
import json
import shutil
import subprocess
import threading
import tkinter as tk
import queue
from datetime import datetime
from pathlib import Path
from tkinter import filedialog, font as tkfont, messagebox

try:
    from .catalog import DEVICES, PREMADE_CONFIGS, ROOT, DeviceDefinition
    from .bridge import BridgeApplyError, EnlinkModbusBridge, load_bridge_table, write_bridge_table
    from .discovery import DiscoveryService, DiscoverySnapshot, PortInfo
    from .enlink import capture_page_windows, parse_sensor_readings
    from .firmware import validate_firmware_manifest
    from .regions import DEFAULT_REGION, RADIO_REGIONS
    from .registers import REGISTER_MAPS
except ImportError:  # Direct script execution
    from catalog import DEVICES, PREMADE_CONFIGS, ROOT, DeviceDefinition
    from bridge import BridgeApplyError, EnlinkModbusBridge, load_bridge_table, write_bridge_table
    from discovery import DiscoveryService, DiscoverySnapshot, PortInfo
    from enlink import capture_page_windows, parse_sensor_readings
    from firmware import validate_firmware_manifest
    from regions import DEFAULT_REGION, RADIO_REGIONS
    from registers import REGISTER_MAPS


BG = "#F7F8FA"
SURFACE = "#FFFFFF"
INK = "#20242A"
MUTED = "#69717D"
OUTLINE = "#D9DEE5"
SOFT = "#EEF1F5"
ACCENT = "#3F6F9F"
GOOD = "#2F855A"


def rounded_rect(canvas: tk.Canvas, x1: float, y1: float, x2: float, y2: float,
                 radius: float = 18, **kwargs) -> int:
    points = [
        x1 + radius, y1, x2 - radius, y1, x2, y1, x2, y1 + radius,
        x2, y2 - radius, x2, y2, x2 - radius, y2, x1 + radius, y2,
        x1, y2, x1, y2 - radius, x1, y1 + radius, x1, y1,
    ]
    return canvas.create_polygon(points, smooth=True, splinesteps=24, **kwargs)


class DeviceIcon:
    @staticmethod
    def draw(canvas: tk.Canvas, kind: str, cx: float, cy: float, color: str) -> None:
        if kind == "bridge":
            canvas.create_rectangle(cx - 25, cy - 20, cx + 25, cy + 24,
                                    outline=color, width=3)
            canvas.create_line(cx + 14, cy - 20, cx + 25, cy - 37,
                               fill=color, width=3)
            for radius in (10, 18):
                canvas.create_arc(cx + 15 - radius, cy - 43 - radius,
                                  cx + 15 + radius, cy - 43 + radius,
                                  start=25, extent=130, style="arc", outline=color, width=2)
            canvas.create_oval(cx - 15, cy - 8, cx - 7, cy, fill=color, outline="")
            canvas.create_oval(cx - 2, cy - 8, cx + 6, cy, fill=color, outline="")
        elif kind == "humidity":
            points = [cx, cy - 34, cx - 23, cy + 4, cx - 19, cy + 24,
                      cx, cy + 34, cx + 19, cy + 24, cx + 23, cy + 4]
            canvas.create_polygon(points, smooth=True, fill="", outline=color, width=3)
            canvas.create_arc(cx - 12, cy + 2, cx + 13, cy + 23,
                              start=195, extent=125, style="arc", outline=color, width=3)
        elif kind == "meter":
            canvas.create_rectangle(cx - 28, cy - 30, cx + 28, cy + 30,
                                    outline=color, width=3)
            canvas.create_arc(cx - 18, cy - 19, cx + 18, cy + 17,
                              start=20, extent=140, style="arc", outline=color, width=3)
            canvas.create_line(cx, cy, cx + 11, cy - 11, fill=color, width=3)
            canvas.create_line(cx - 14, cy + 21, cx + 14, cy + 21, fill=color, width=3)
        elif kind == "air_quality":
            canvas.create_oval(cx - 30, cy - 30, cx + 30, cy + 30,
                               outline=color, width=3)
            for dx, dy, radius in ((-11, -8, 4), (8, -13, 3), (12, 8, 5), (-8, 13, 3)):
                canvas.create_oval(cx + dx - radius, cy + dy - radius,
                                   cx + dx + radius, cy + dy + radius,
                                   fill=color, outline="")
        else:
            canvas.create_rectangle(cx - 27, cy - 18, cx + 17, cy + 18,
                                    outline=color, width=3)
            canvas.create_line(cx + 17, cy - 8, cx + 31, cy - 8,
                               cx + 31, cy + 8, cx + 17, cy + 8,
                               fill=color, width=3)
            canvas.create_line(cx - 18, cy - 7, cx + 8, cy - 7, fill=color, width=2)
            canvas.create_line(cx - 18, cy + 2, cx + 8, cy + 2, fill=color, width=2)


class App(tk.Tk):
    def __init__(self) -> None:
        super().__init__()
        self.title("ExactAire Modbus Configurator")
        self.geometry("1180x760")
        self.minsize(960, 640)
        self.configure(bg=BG)
        self.option_add("*Font", "{Segoe UI} 10")
        self.heading = tkfont.Font(family="Segoe UI", size=24, weight="bold")
        self.title_font = tkfont.Font(family="Segoe UI", size=13, weight="bold")
        self.small_bold = tkfont.Font(family="Segoe UI", size=9, weight="bold")
        self.ports: list[PortInfo] = []
        self.classified: dict[str, PortInfo] = {}
        self.instruments: dict[str, object] = {}
        self.adapter_status = "absent"
        self.live_readings: dict[str, dict[str, tuple[float | int | str, str]]] = {}
        self.active_key: str | None = None
        self.discovery_queue: queue.Queue[DiscoverySnapshot] = queue.Queue()
        self.action_queue: queue.Queue[tuple[str, object]] = queue.Queue()
        self.discovery = DiscoveryService(self.discovery_queue.put, interval_seconds=1.5)
        self.protocol("WM_DELETE_WINDOW", self.close_app)
        self.show_overview()
        self.discovery.start()
        self.after(100, self.poll_discovery_results)

    def clear(self) -> None:
        for child in self.winfo_children():
            child.destroy()

    def show_overview(self) -> None:
        self.active_key = None
        self.clear()
        header = tk.Frame(self, bg=BG)
        header.pack(fill="x", padx=42, pady=(30, 12))
        tk.Label(header, text="Connected devices", font=self.heading,
                 fg=INK, bg=BG).pack(side="left")
        self.scan_label = tk.Label(header, text="Scanning…", fg=MUTED, bg=BG)
        self.scan_label.pack(side="right", padx=(12, 0))
        tk.Button(header, text="↻  Refresh", command=self.refresh,
                  bg=SURFACE, fg=INK, activebackground=SOFT, relief="flat",
                  padx=16, pady=8, cursor="hand2",
                  highlightthickness=1, highlightbackground=OUTLINE).pack(side="right")

        actions = tk.Frame(self, bg=BG)
        actions.pack(fill="x", padx=42, pady=(8, 0))
        self.action_button(actions, "Back up configuration", self.backup_configuration).pack(side="left")
        self.action_button(actions, "Choose pre-made config", self.choose_configuration).pack(side="left", padx=10)

        tk.Label(self, text="Select a device to view its information and readouts.",
                 fg=MUTED, bg=BG).pack(anchor="w", padx=44)

        self.canvas = tk.Canvas(self, bg=BG, highlightthickness=0, height=390)
        self.canvas.pack(fill="both", expand=True, padx=36, pady=(18, 0))

        profiles = tk.Frame(self, bg=BG)
        profiles.pack(fill="x", padx=42, pady=(4, 32))
        tk.Label(profiles, text="READY-TO-TEST PROFILES", font=self.small_bold,
                 fg=MUTED, bg=BG).pack(anchor="w", pady=(0, 10))
        row = tk.Frame(profiles, bg=BG)
        row.pack(fill="x")
        self.profile_cards: list[tk.Frame] = []
        for key in ("hmd65", "wattnode"):
            self.profile_cards.append(self.make_profile_card(row, DEVICES[key]))

        self.refresh()

    def refresh(self) -> None:
        self.scan_label.configure(text="Scanning serial interfaces…")
        self.discovery.retry_direct_probe()

    def poll_discovery_results(self) -> None:
        latest = None
        try:
            while True:
                latest = self.discovery_queue.get_nowait()
        except queue.Empty:
            pass
        if latest:
            self.apply_discovery_snapshot(latest)
        try:
            while True:
                action, payload = self.action_queue.get_nowait()
                self.apply_device_action(action, payload)
        except queue.Empty:
            pass
        self.after(100, self.poll_discovery_results)

    def apply_device_action(self, action: str, payload: object) -> None:
        if action == "iaq_live":
            self.live_readings["iaq_plus"] = payload  # type: ignore[assignment]
            if self.active_key == "iaq_plus":
                self.show_detail("iaq_plus")
        elif action == "backup_complete":
            messagebox.showinfo("Backup created", f"Saved IAQ Plus backup to:\n{payload}", parent=self)
        elif action == "bridge_backup_complete":
            messagebox.showinfo("Backup created", f"Saved the bridge's native TSV export to:\n{payload}", parent=self)
        elif action == "error":
            messagebox.showerror("Device action failed", str(payload), parent=self)
        elif action == "bridge_apply_complete":
            result, backup_path = payload  # type: ignore[misc]
            messagebox.showinfo(
                "Bridge configuration applied",
                f"Configuration was written and read back successfully.\n\n"
                f"Read All: {result.read_summary}\nBackup: {backup_path}",
                parent=self,
            )
            self.refresh()
        elif action == "bridge_apply_failed":
            message, backup_path, rollback_ok = payload  # type: ignore[misc]
            recovery = ("Previous table restored successfully." if rollback_ok else
                        "Success was not verified. Preserve power and USB, then restore from the saved live backup.")
            messagebox.showerror(
                "Bridge configuration failed",
                f"{message}\n\n{recovery}\nBackup: {backup_path}",
                parent=self,
            )
            self.refresh()

    def apply_discovery_snapshot(self, snapshot: DiscoverySnapshot) -> None:
        self.ports = list(snapshot.ports)
        self.classified = snapshot.classified
        self.adapter_status = snapshot.adapter_status
        self.instruments = {getattr(item, "key"): item for item in snapshot.instruments}
        count = len(self.ports)
        if self.active_key is None and hasattr(self, "canvas") and self.canvas.winfo_exists():
            self.draw_topology()
            self.scan_label.configure(
                text=(f"{count} interface{'s' if count != 1 else ''} · {snapshot.message} · "
                      f"checked {datetime.now().strftime('%H:%M:%S')}"),
            )

    def draw_topology(self) -> None:
        self.canvas.delete("all")
        width = max(self.canvas.winfo_width(), 900)
        center_y = 185
        nodes: list[tuple[str, float]] = []
        disabled_keys: set[str] = set()
        linked_pair = False
        detected_key = next(iter(self.instruments), None)
        if "bridge" in self.classified:
            if "adapter" in self.classified:
                nodes = [("bridge", width * 0.22), ("dpt146", width * 0.50),
                         ("adapter", width * 0.80)]
                disabled_keys.add("adapter")
            else:
                nodes = [("bridge", width * 0.34), ("dpt146", width * 0.66)]
            linked_pair = True
        elif "synetica_usb" in self.classified and "adapter" in self.classified:
            nodes = [("synetica_usb", width * 0.34), ("adapter", width * 0.66)]
            disabled_keys.add("adapter")
        elif "synetica_usb" in self.classified:
            nodes = [("synetica_usb", width * 0.50)]
        elif "iaq_plus" in self.classified:
            nodes = [("iaq_plus", width * 0.50)]
        elif "adapter" in self.classified and detected_key:
            nodes = [("adapter", width * 0.34), (detected_key, width * 0.66)]
            linked_pair = True
        elif "adapter" in self.classified:
            nodes = [("adapter", width * 0.50)]

        if self.adapter_status in ("blocked_by_bridge", "no_response"):
            disabled_keys.add("adapter")

        if not nodes:
            rounded_rect(self.canvas, width / 2 - 245, center_y - 65,
                         width / 2 + 245, center_y + 65, fill=SURFACE, outline=OUTLINE)
            self.canvas.create_text(width / 2, center_y - 15,
                                    text="No supported serial hardware detected",
                                    font=self.title_font, fill=INK)
            self.canvas.create_text(width / 2, center_y + 20,
                                    text="Connect the bridge or USB-COMi-TB, then refresh.",
                                    fill=MUTED)
            if self.ports:
                self.canvas.create_text(width / 2, center_y + 48,
                                        text="Other ports: " + ", ".join(p.port for p in self.ports),
                                        fill=MUTED, font=("Segoe UI", 9))
            return

        if linked_pair:
            self.canvas.create_line(nodes[0][1] + 112, center_y,
                                    nodes[1][1] - 112, center_y,
                                    fill="#AAB2BD", width=3)
            midpoint = (nodes[0][1] + nodes[1][1]) / 2
            self.canvas.create_oval(midpoint - 5, center_y - 5,
                                    midpoint + 5, center_y + 5,
                                    fill=GOOD, outline=BG, width=3)
            self.canvas.create_text(midpoint, center_y + 23,
                                    text="RS-485", fill=MUTED, font=("Segoe UI", 9))

        for key, x in nodes:
            port = self.classified.get(key)
            instrument = self.instruments.get(key)
            disabled = key in disabled_keys
            if disabled:
                if self.adapter_status == "blocked_by_bridge":
                    connection = "Unavailable as Modbus master\nAnother master interface may be active"
                else:
                    connection = "No Modbus response\nNothing connected, or another master may be active"
            elif port:
                connection = port.port
            elif instrument:
                connection = f"Detected on {getattr(instrument, 'port')} · {getattr(instrument, 'confidence')} confidence"
            else:
                connection = "Configured behind bridge"
            self.draw_device_card(DEVICES[key], x, center_y, connection, disabled)

    def draw_device_card(self, device: DeviceDefinition, cx: float, cy: float,
                         connection: str, disabled: bool = False) -> None:
        x1, y1, x2, y2 = cx - 112, cy - 132, cx + 112, cy + 132
        card_fill = "#FFF7D6" if disabled else SURFACE
        icon_fill = "#FCE9A9" if disabled else SOFT
        device_color = "#B7791F" if disabled else device.color
        text_color = "#744210" if disabled else INK
        items: list[int] = []
        items.append(rounded_rect(self.canvas, x1, y1, x2, y2,
                                  fill=card_fill, outline=OUTLINE, width=1))
        items.append(self.canvas.create_oval(cx - 49, cy - 94, cx + 49, cy + 4,
                                             fill=icon_fill, outline=""))
        DeviceIcon.draw(self.canvas, device.kind, cx, cy - 45, device_color)
        items.append(self.canvas.create_text(cx, cy + 28, text=device.name,
                                             font=self.title_font, fill=text_color, width=195))
        items.append(self.canvas.create_text(cx, cy + 58, text=device.subtitle,
                                             fill=MUTED, width=195))
        items.append(self.canvas.create_text(cx, cy + 94, text=connection,
                                             fill="#975A16" if disabled else GOOD,
                                             font=self.small_bold, width=195))
        tag = f"device-{device.key}"
        for item in self.canvas.find_enclosed(x1 - 2, y1 - 2, x2 + 2, y2 + 2):
            self.canvas.addtag_withtag(tag, item)
        self.canvas.tag_bind(tag, "<Button-1>", lambda _event, key=device.key: self.show_detail(key))
        self.canvas.tag_bind(tag, "<Enter>", lambda _event: self.canvas.configure(cursor="hand2"))
        self.canvas.tag_bind(tag, "<Leave>", lambda _event: self.canvas.configure(cursor=""))

    def make_profile_card(self, parent: tk.Widget, device: DeviceDefinition) -> tk.Frame:
        card = tk.Frame(parent, bg=SURFACE, highlightthickness=1,
                        highlightbackground=OUTLINE, cursor="hand2")
        card.pack(side="left", fill="x", expand=True, padx=(0, 14))
        inner = tk.Frame(card, bg=SURFACE, padx=18, pady=14)
        inner.pack(fill="both", expand=True)
        tk.Label(inner, text=device.name, font=self.title_font,
                 fg=INK, bg=SURFACE).pack(anchor="w")
        tk.Label(inner, text=device.status, fg=device.color,
                 bg=SURFACE).pack(anchor="w", pady=(4, 0))
        for widget in (card, inner, *inner.winfo_children()):
            widget.bind("<Button-1>", lambda _event, key=device.key: self.show_detail(key))
        return card

    @staticmethod
    def action_button(parent: tk.Widget, text: str, command) -> tk.Button:
        return tk.Button(parent, text=text, command=command, bg=SURFACE, fg=INK,
                         activebackground=SOFT, relief="flat", padx=16, pady=9,
                         cursor="hand2", highlightthickness=1,
                         highlightbackground=OUTLINE)

    def show_detail(self, key: str) -> None:
        self.active_key = key
        self.clear()
        device = DEVICES[key]
        top = tk.Frame(self, bg=BG)
        top.pack(fill="x", padx=36, pady=(26, 8))
        tk.Button(top, text="←", command=self.show_overview, font=("Segoe UI", 20),
                  fg=INK, bg=BG, activebackground=SOFT, relief="flat",
                  cursor="hand2", width=2).pack(side="left")
        title_box = tk.Frame(top, bg=BG)
        title_box.pack(side="left", padx=14)
        tk.Label(title_box, text=device.name, font=self.heading,
                 fg=INK, bg=BG).pack(anchor="w")
        tk.Label(title_box, text=device.subtitle, fg=MUTED, bg=BG).pack(anchor="w")

        body = tk.Frame(self, bg=BG)
        body.pack(fill="both", expand=True, padx=44, pady=18)
        left = tk.Frame(body, bg=BG)
        left.pack(side="left", fill="both", expand=True, padx=(0, 16))
        right = tk.Frame(body, bg=BG, width=330)
        right.pack(side="right", fill="y")
        right.pack_propagate(False)

        status = self.connection_status(key)
        hero = tk.Frame(left, bg=SURFACE, padx=24, pady=22,
                        highlightthickness=1, highlightbackground=OUTLINE)
        hero.pack(fill="x")
        tk.Label(hero, text=device.status.upper(), font=self.small_bold,
                 fg=device.color, bg=SURFACE).pack(anchor="w")
        tk.Label(hero, text=device.description, wraplength=610, justify="left",
                 fg=INK, bg=SURFACE, font=("Segoe UI", 12)).pack(anchor="w", pady=(9, 16))
        tk.Label(hero, text=status, fg=GOOD if "Connected" in status else MUTED,
                 bg=SURFACE).pack(anchor="w")

        tk.Label(left, text="READOUTS", font=self.small_bold,
                 fg=MUTED, bg=BG).pack(anchor="w", pady=(24, 10))
        grid = tk.Frame(left, bg=BG)
        grid.pack(fill="both", expand=True)
        live = dict(getattr(self.instruments.get(key), "readings", {}))
        live.update(self.live_readings.get(key, {}))
        for index, item in enumerate(device.readouts):
            box = tk.Frame(grid, bg=SURFACE, padx=18, pady=15,
                           highlightthickness=1, highlightbackground=OUTLINE)
            box.grid(row=index // 2, column=index % 2, sticky="nsew", padx=(0, 12), pady=(0, 12))
            grid.columnconfigure(index % 2, weight=1)
            tk.Label(box, text=item.label, fg=MUTED, bg=SURFACE).pack(anchor="w")
            live_value = live.get(item.label)
            raw_value, raw_unit = live_value if live_value else (item.value, item.unit)
            value = f"{raw_value} {raw_unit}".strip()
            tk.Label(box, text=value, fg=INK, bg=SURFACE,
                     font=("Segoe UI", 18, "bold")).pack(anchor="w", pady=(6, 2))
            quality = "Live auto-identified reading" if live_value else item.quality
            tk.Label(box, text=quality, fg=GOOD if live_value or "validated" in quality.lower() else MUTED,
                     bg=SURFACE, font=("Segoe UI", 8)).pack(anchor="w")

        facts = tk.Frame(right, bg=SURFACE, padx=22, pady=22,
                         highlightthickness=1, highlightbackground=OUTLINE)
        facts.pack(fill="both", expand=True)
        tk.Label(facts, text="DEVICE INFORMATION", font=self.small_bold,
                 fg=MUTED, bg=SURFACE).pack(anchor="w", pady=(0, 12))
        for label, value in device.facts:
            row = tk.Frame(facts, bg=SURFACE)
            row.pack(fill="x", pady=7)
            tk.Label(row, text=label, fg=MUTED, bg=SURFACE).pack(anchor="w")
            tk.Label(row, text=value, fg=INK, bg=SURFACE,
                     font=("Segoe UI", 10, "bold"), wraplength=270,
                     justify="left").pack(anchor="w", pady=(2, 0))
        if device.artifact:
            tk.Button(facts, text="Open configuration artifact",
                      command=lambda: self.open_artifact(device.artifact),
                      bg=ACCENT, fg="white", activebackground="#315A82",
                      activeforeground="white", relief="flat", padx=14,
                      pady=10, cursor="hand2").pack(side="bottom", fill="x")
        tk.Button(facts, text="Help: setup & troubleshooting",
                  command=lambda: self.show_help(device), bg=SURFACE, fg=INK,
                  activebackground=SOFT, relief="flat", padx=14, pady=10,
                  cursor="hand2", highlightthickness=1,
                  highlightbackground=OUTLINE).pack(side="bottom", fill="x", pady=(0, 10))

        if key in REGISTER_MAPS:
            tk.Button(facts, text="Register table",
                      command=lambda: self.show_registers(key), bg=SURFACE, fg=INK,
                      activebackground=SOFT, relief="flat", padx=14, pady=10,
                      cursor="hand2", highlightthickness=1,
                      highlightbackground=OUTLINE).pack(side="bottom", fill="x", pady=(0, 10))

        if key == "bridge":
            tk.Button(facts, text="Choose pre-made config",
                      command=self.choose_configuration, bg=SURFACE, fg=INK,
                      activebackground=SOFT, relief="flat", padx=14, pady=10,
                      cursor="hand2", highlightthickness=1,
                      highlightbackground=OUTLINE).pack(side="bottom", fill="x", pady=(0, 10))
            tk.Button(facts, text="Back up configuration",
                      command=self.backup_configuration, bg=SURFACE, fg=INK,
                      activebackground=SOFT, relief="flat", padx=14, pady=10,
                      cursor="hand2", highlightthickness=1,
                      highlightbackground=OUTLINE).pack(side="bottom", fill="x", pady=(0, 10))
        elif key == "iaq_plus":
            tk.Button(facts, text="Refresh live readings",
                      command=self.refresh_iaq_live, bg=ACCENT, fg="white",
                      activebackground="#315A82", activeforeground="white",
                      relief="flat", padx=14, pady=10, cursor="hand2").pack(
                          side="bottom", fill="x", pady=(0, 10))
            tk.Button(facts, text="Back up device configuration",
                      command=self.backup_iaq_configuration, bg=SURFACE, fg=INK,
                      activebackground=SOFT, relief="flat", padx=14, pady=10,
                      cursor="hand2", highlightthickness=1,
                      highlightbackground=OUTLINE).pack(side="bottom", fill="x", pady=(0, 10))
            tk.Button(facts, text="Choose radio profile",
                      command=self.choose_iaq_region, bg=SURFACE, fg=INK,
                      activebackground=SOFT, relief="flat", padx=14, pady=10,
                      cursor="hand2", highlightthickness=1,
                      highlightbackground=OUTLINE).pack(side="bottom", fill="x", pady=(0, 10))
            tk.Button(facts, text="Firmware update package…",
                      command=self.inspect_iaq_firmware_package, bg=SURFACE, fg=INK,
                      activebackground=SOFT, relief="flat", padx=14, pady=10,
                      cursor="hand2", highlightthickness=1,
                      highlightbackground=OUTLINE).pack(side="bottom", fill="x", pady=(0, 10))

    def show_registers(self, key: str) -> None:
        self.active_key = key
        self.clear()
        device = DEVICES[key]
        top = tk.Frame(self, bg=BG)
        top.pack(fill="x", padx=36, pady=(26, 8))
        tk.Button(top, text="←", command=lambda: self.show_detail(key),
                  font=("Segoe UI", 20), fg=INK, bg=BG,
                  activebackground=SOFT, relief="flat", cursor="hand2",
                  width=2).pack(side="left")
        title_box = tk.Frame(top, bg=BG)
        title_box.pack(side="left", padx=14)
        tk.Label(title_box, text=f"{device.name} registers", font=self.heading,
                 fg=INK, bg=BG).pack(anchor="w")
        tk.Label(title_box,
                 text="Logical addresses follow the manual; PDU addresses are the zero-based values sent on the wire.",
                 fg=MUTED, bg=BG).pack(anchor="w")

        live = {item.label: (item.value, item.unit) for item in device.readouts}
        live.update(dict(getattr(self.instruments.get(key), "readings", {})))
        live.update(self.live_readings.get(key, {}))

        shell = tk.Frame(self, bg=SURFACE, highlightthickness=1,
                         highlightbackground=OUTLINE)
        shell.pack(fill="both", expand=True, padx=44, pady=(12, 34))
        columns = ("name", "logical", "pdu", "type", "access", "value", "decoded", "unit", "description")
        headings = {
            "name": "Register / value", "logical": "Manual", "pdu": "PDU",
            "type": "Type", "access": "Access", "value": "Readout",
            "decoded": "Decoded meaning", "unit": "Unit", "description": "Description",
        }
        widths = {"name": 170, "logical": 90, "pdu": 90, "type": 80,
                  "access": 58, "value": 105, "decoded": 290,
                  "unit": 72, "description": 370}

        canvas = tk.Canvas(shell, bg=SURFACE, highlightthickness=0)
        vertical = tk.Scrollbar(shell, orient="vertical", command=canvas.yview)
        horizontal = tk.Scrollbar(shell, orient="horizontal", command=canvas.xview)
        canvas.configure(yscrollcommand=vertical.set, xscrollcommand=horizontal.set)
        canvas.grid(row=0, column=0, sticky="nsew")
        vertical.grid(row=0, column=1, sticky="ns")
        horizontal.grid(row=1, column=0, sticky="ew")
        shell.rowconfigure(0, weight=1)
        shell.columnconfigure(0, weight=1)

        table = tk.Frame(canvas, bg=OUTLINE)
        window = canvas.create_window((0, 0), window=table, anchor="nw")
        for index, column in enumerate(columns):
            table.columnconfigure(index, minsize=widths[column])
            tk.Label(table, text=headings[column], bg=SOFT, fg=INK,
                     font=self.small_bold, anchor="w", justify="left",
                     padx=8, pady=9, width=1,
                     wraplength=widths[column] - 16).grid(
                         row=0, column=index, sticky="nsew", padx=(0, 1), pady=(0, 1))

        for row_index, register in enumerate(REGISTER_MAPS[key], start=1):
            current = live.get(register.readout_label) if register.readout_label else None
            value = "—"
            unit = register.unit
            if current:
                raw_value, raw_unit = current
                value = str(raw_value)
                unit = raw_unit or unit
            decoded = register.decode(value if current else None)
            values = (
                register.name, register.logical, register.pdu, register.data_type,
                register.access, value, decoded, unit, register.description,
            )
            row_color = SURFACE if row_index % 2 else BG
            for column_index, (column, cell_value) in enumerate(zip(columns, values)):
                tk.Label(table, text=cell_value, bg=row_color, fg=INK,
                         anchor="nw", justify="left", padx=8, pady=8,
                         width=1, wraplength=widths[column] - 16).grid(
                             row=row_index, column=column_index, sticky="nsew",
                             padx=(0, 1), pady=(0, 1))

        def update_scroll_region(_event=None) -> None:
            table.update_idletasks()
            canvas.configure(scrollregion=canvas.bbox("all"))
            canvas.itemconfigure(window, width=max(table.winfo_reqwidth(), canvas.winfo_width()))

        table.bind("<Configure>", update_scroll_region)
        canvas.bind("<Configure>", update_scroll_region)
        canvas.bind("<MouseWheel>",
                    lambda event: canvas.yview_scroll(-1 * int(event.delta / 120), "units"))
        update_scroll_region()

    def connection_status(self, key: str) -> str:
        instrument = self.instruments.get(key)
        if instrument:
            if key == "iaq_plus":
                return (f"Auto-identified on {getattr(instrument, 'port')} · firmware "
                        f"{getattr(instrument, 'firmware_code')} {getattr(instrument, 'firmware')} · "
                        f"{getattr(instrument, 'region')}")
            return (f"Auto-identified on {getattr(instrument, 'port')} · slave {getattr(instrument, 'slave_id')} · "
                    f"{getattr(instrument, 'serial_format')} · {getattr(instrument, 'confidence')} confidence")
        if key in self.classified:
            port = self.classified[key]
            return f"Connected on {port.port} · {port.description}"
        if key == "dpt146" and "bridge" in self.classified:
            return "Configured behind the connected bridge · readouts shown are last validated"
        if key in ("hmd65", "wattnode"):
            return "Profile prepared · physical hardware not detected"
        return "Not currently detected · showing saved project information"

    def close_app(self) -> None:
        self.discovery.stop()
        self.destroy()

    def iaq_port(self) -> str | None:
        instrument = self.instruments.get("iaq_plus")
        if instrument:
            return str(getattr(instrument, "port"))
        port = self.classified.get("iaq_plus")
        return port.port if port else None

    def refresh_iaq_live(self) -> None:
        port = self.iaq_port()
        if not port:
            messagebox.showwarning("Sensor not connected", "Connect the IAQ Plus USB configuration port first.", parent=self)
            return

        def worker() -> None:
            try:
                readings = parse_sensor_readings(capture_page_windows(port, "configure"))
                if not readings:
                    raise RuntimeError("The configuration page did not contain recognizable sensor readings")
                self.action_queue.put(("iaq_live", readings))
            except Exception as exc:  # Hardware/console boundary
                self.action_queue.put(("error", exc))

        threading.Thread(target=worker, daemon=True, name="iaq-live-readings").start()

    def backup_iaq_configuration(self) -> None:
        port = self.iaq_port()
        if not port:
            messagebox.showwarning("Sensor not connected", "Connect the IAQ Plus USB configuration port first.", parent=self)
            return
        stamp = datetime.now().strftime("%Y%m%d-%H%M%S")
        destination = filedialog.asksaveasfilename(
            parent=self, title="Save IAQ Plus configuration backup",
            defaultextension=".json", initialfile=f"enlink-iaq-plus-{stamp}.json",
            filetypes=[("JSON configuration backup", "*.json")],
        )
        if not destination:
            return

        instrument = self.instruments.get("iaq_plus")

        def worker() -> None:
            try:
                payload = {
                    "schema_version": 1,
                    "captured_at": datetime.now().astimezone().isoformat(),
                    "identity": {
                        name: getattr(instrument, name, None)
                        for name in ("firmware_code", "firmware", "region", "dev_eui")
                    },
                    "radio_profile_default": DEFAULT_REGION,
                    "pages": {
                        page: capture_page_windows(port, page, redact_secrets=False)
                        for page in ("quick_start", "radio", "configure")
                    },
                }
                Path(destination).write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
                self.action_queue.put(("backup_complete", destination))
            except Exception as exc:  # Hardware/console boundary
                self.action_queue.put(("error", exc))

        threading.Thread(target=worker, daemon=True, name="iaq-config-backup").start()

    def choose_iaq_region(self) -> None:
        dialog = tk.Toplevel(self)
        dialog.title("Choose IAQ Plus radio profile")
        dialog.geometry("620x390")
        dialog.configure(bg=BG)
        dialog.transient(self)
        dialog.grab_set()
        tk.Label(dialog, text="Choose radio profile", font=("Segoe UI", 19, "bold"),
                 fg=INK, bg=BG).pack(anchor="w", padx=28, pady=(24, 4))
        tk.Label(dialog, text="Preview only — this prototype does not change the device region.",
                 fg=MUTED, bg=BG).pack(anchor="w", padx=28, pady=(0, 16))
        selected = tk.StringVar(value=DEFAULT_REGION)
        for key, region in RADIO_REGIONS.items():
            row = tk.Frame(dialog, bg=SURFACE, padx=14, pady=12,
                           highlightthickness=1, highlightbackground=OUTLINE)
            row.pack(fill="x", padx=28, pady=5)
            tk.Radiobutton(row, variable=selected, value=key, bg=SURFACE,
                           activebackground=SURFACE, selectcolor=SURFACE,
                           state="normal" if region.enabled else "disabled").pack(side="left")
            labels = tk.Frame(row, bg=SURFACE)
            labels.pack(side="left", padx=8)
            tk.Label(labels, text=region.display_name, font=self.title_font,
                     fg=INK if region.enabled else MUTED, bg=SURFACE).pack(anchor="w")
            state = "Enabled default" if region.enabled else "Future stub · disabled"
            tk.Label(labels, text=f"{region.frequency_mhz} MHz · {state}",
                     fg=GOOD if region.enabled else MUTED, bg=SURFACE).pack(anchor="w")
        tk.Button(dialog, text="Close", command=dialog.destroy, bg=ACCENT, fg="white",
                  relief="flat", padx=20, pady=9).pack(side="bottom", anchor="e", padx=28, pady=24)

    def inspect_iaq_firmware_package(self) -> None:
        instrument = self.instruments.get("iaq_plus")
        if instrument is None:
            messagebox.showwarning("Sensor not connected", "Connect and identify the IAQ Plus before selecting firmware.", parent=self)
            return
        manifest = filedialog.askopenfilename(
            parent=self, title="Select vendor firmware manifest",
            filetypes=[("Firmware manifest", "*.json")],
        )
        if not manifest:
            return
        region_text = str(getattr(instrument, "region", ""))
        region_key = "us915_hybrid_fsb1" if "915" in region_text else "eu868" if "868" in region_text else "unknown"
        result = validate_firmware_manifest(
            Path(manifest), product_family="Synetica enLink IAQ Plus",
            firmware_code=str(getattr(instrument, "firmware_code")),
            current_version=str(getattr(instrument, "firmware")), region_key=region_key,
        )
        lines = [
            f"Connected firmware: {getattr(instrument, 'firmware_code')} {getattr(instrument, 'firmware')}",
            f"Detected region: {region_key}",
            f"Target firmware: {result.target_version or 'unknown'}",
            "",
        ]
        if result.valid:
            lines.extend((
                "Package validation passed.",
                "",
                "Flashing is still disabled until the vendor update and recovery procedures are implemented.",
            ))
        else:
            lines.append("Package blocked:")
            lines.extend(f"• {error}" for error in result.errors)
        if result.warnings:
            lines.append("")
            lines.extend(f"Warning: {warning}" for warning in result.warnings)
        messagebox.showinfo("Firmware package inspection", "\n".join(lines), parent=self)

    def backup_configuration(self) -> None:
        bridge_port = self.classified.get("bridge")
        instrument = self.instruments.get("bridge")
        if bridge_port is None or instrument is None:
            messagebox.showwarning(
                "Bridge not ready",
                "Connect and identify the ENL-MOD-32 before exporting its configuration.",
                parent=self,
            )
            return
        stamp = datetime.now().strftime("%Y%m%d-%H%M%S")
        destination = filedialog.asksaveasfilename(
            parent=self,
            title="Save native bridge configuration export",
            defaultextension=".tsv",
            initialfile=f"enl-mod-32-export-{stamp}.tsv",
            filetypes=[("Tab-delimited configuration", "*.tsv")],
        )
        if not destination:
            return
        self.scan_label.configure(text="Exporting native bridge table…")

        def worker() -> None:
            if not self.discovery.suspend():
                self.action_queue.put(("error", "Could not obtain exclusive access to bridge discovery."))
                self.discovery.resume()
                return
            try:
                rows = EnlinkModbusBridge(bridge_port.port).export_verified()
                write_bridge_table(Path(destination), rows)
                self.action_queue.put(("bridge_backup_complete", destination))
            except Exception as exc:
                self.action_queue.put(("error", exc))
            finally:
                self.discovery.resume()

        threading.Thread(target=worker, daemon=True, name="bridge-config-export").start()

    def choose_configuration(self) -> None:
        dialog = tk.Toplevel(self)
        dialog.title("Choose pre-made configuration")
        dialog.geometry("560x430")
        dialog.resizable(False, False)
        dialog.configure(bg=BG)
        dialog.transient(self)
        dialog.grab_set()
        tk.Label(dialog, text="Choose a pre-made configuration",
                 font=("Segoe UI", 19, "bold"), fg=INK, bg=BG).pack(anchor="w", padx=28, pady=(24, 4))
        bridge_ready = "bridge" in self.classified and "bridge" in self.instruments
        tk.Label(dialog, text=("Review/export a profile, or apply it to the connected firmware 3.6 bridge."
                               if bridge_ready else
                               "Connect and identify a firmware 3.6 bridge to enable programming."),
                 fg=MUTED, bg=BG).pack(anchor="w", padx=28, pady=(0, 16))
        selected = tk.StringVar(value="dpt146")
        for key in ("dpt146", "hmd65", "wattnode"):
            device = DEVICES[key]
            row = tk.Frame(dialog, bg=SURFACE, padx=14, pady=10,
                           highlightthickness=1, highlightbackground=OUTLINE)
            row.pack(fill="x", padx=28, pady=5)
            tk.Radiobutton(row, variable=selected, value=key, bg=SURFACE,
                           activebackground=SURFACE, selectcolor=SURFACE).pack(side="left")
            labels = tk.Frame(row, bg=SURFACE)
            labels.pack(side="left", padx=8)
            tk.Label(labels, text=device.name, font=self.title_font,
                     fg=INK, bg=SURFACE).pack(anchor="w")
            tk.Label(labels, text=device.status, fg=device.color,
                     bg=SURFACE).pack(anchor="w")

        buttons = tk.Frame(dialog, bg=BG)
        buttons.pack(side="bottom", fill="x", padx=28, pady=24)
        tk.Button(buttons, text="Cancel", command=dialog.destroy,
                  bg=SURFACE, fg=INK, relief="flat", padx=18, pady=9,
                  highlightthickness=1, highlightbackground=OUTLINE).pack(side="right")
        tk.Button(buttons, text="Export TSV…",
                  command=lambda: self.copy_configuration(selected.get(), dialog),
                  bg=SURFACE, fg=INK, activebackground=SOFT,
                  activeforeground="white", relief="flat", padx=18,
                  pady=9, cursor="hand2").pack(side="right", padx=10)
        tk.Button(buttons, text="Apply to bridge…",
                  command=lambda: self.apply_bridge_configuration(selected.get(), dialog),
                  state="normal" if bridge_ready else "disabled",
                  bg=ACCENT, fg="white", activebackground="#315A82",
                  activeforeground="white", relief="flat", padx=18,
                  pady=9, cursor="hand2").pack(side="right")

    def copy_configuration(self, key: str, dialog: tk.Toplevel) -> None:
        source = PREMADE_CONFIGS[key]
        destination = filedialog.asksaveasfilename(
            parent=dialog,
            title="Copy pre-made configuration",
            defaultextension=".tsv",
            initialfile=source.name,
            filetypes=[("Tab-delimited configuration", "*.tsv")],
        )
        if not destination:
            return
        try:
            shutil.copy2(source, destination)
            dialog.destroy()
            messagebox.showinfo(
                "Configuration copied",
                "Review the slave ID and serial settings before importing it into a bridge.",
                parent=self,
            )
        except OSError as exc:
            messagebox.showerror("Copy failed", str(exc), parent=dialog)

    def apply_bridge_configuration(self, key: str, dialog: tk.Toplevel) -> None:
        bridge_port = self.classified.get("bridge")
        instrument = self.instruments.get("bridge")
        if bridge_port is None or instrument is None:
            messagebox.showwarning("Bridge not ready", "Connect and identify the ENL-MOD-32 first.", parent=dialog)
            return
        if str(getattr(instrument, "firmware", "")) != "3.6":
            messagebox.showerror("Unsupported firmware", "Only validated ENL-MOD-32 firmware 3.6 can be programmed.", parent=dialog)
            return
        try:
            rows = load_bridge_table(PREMADE_CONFIGS[key])
        except (OSError, ValueError) as exc:
            messagebox.showerror("Invalid configuration", str(exc), parent=dialog)
            return
        stamp = datetime.now().strftime("%Y%m%d-%H%M%S")
        backup_path_text = filedialog.asksaveasfilename(
            parent=dialog, title="Save current bridge table before programming",
            defaultextension=".tsv", initialfile=f"enl-mod-32-live-backup-{stamp}.tsv",
            filetypes=[("Tab-delimited configuration", "*.tsv")],
        )
        if not backup_path_text:
            return
        device = DEVICES[key]
        if not messagebox.askyesno(
            "Apply configuration",
            f"Program {device.name} onto {getattr(instrument, 'display_name')} firmware 3.6?\n\n"
            f"The current point table will be backed up, replaced, exported for comparison, "
            f"and tested with Read All Data Points.",
            parent=dialog,
        ):
            return
        dialog.destroy()
        self.scan_label.configure(text="Programming bridge — do not disconnect power or USB…")
        backup_path = Path(backup_path_text)

        def worker() -> None:
            if not self.discovery.suspend():
                self.action_queue.put(("bridge_apply_failed", (
                    "Could not obtain exclusive access to bridge discovery.", backup_path, True,
                )))
                self.discovery.resume()
                return
            try:
                bridge = EnlinkModbusBridge(bridge_port.port)
                result = bridge.apply_verified(rows)
                write_bridge_table(backup_path, result.backup_rows)
                self.action_queue.put(("bridge_apply_complete", (result, backup_path)))
            except BridgeApplyError as exc:
                try:
                    write_bridge_table(backup_path, exc.backup_rows)
                except OSError:
                    pass
                self.action_queue.put(("bridge_apply_failed", (str(exc), backup_path, exc.rollback_ok)))
            except Exception as exc:
                self.action_queue.put(("bridge_apply_failed", (str(exc), backup_path, False)))
            finally:
                self.discovery.resume()

        threading.Thread(target=worker, daemon=True, name="bridge-config-apply").start()

    def show_help(self, device: DeviceDefinition) -> None:
        dialog = tk.Toplevel(self)
        dialog.title(f"{device.name} help")
        dialog.geometry("650x540")
        dialog.configure(bg=BG)
        dialog.transient(self)
        tk.Label(dialog, text=f"{device.name} help", font=("Segoe UI", 20, "bold"),
                 fg=INK, bg=BG).pack(anchor="w", padx=30, pady=(26, 4))
        tk.Label(dialog, text="Basic setup and troubleshooting",
                 fg=MUTED, bg=BG).pack(anchor="w", padx=30, pady=(0, 18))
        content = tk.Frame(dialog, bg=SURFACE, padx=24, pady=20,
                           highlightthickness=1, highlightbackground=OUTLINE)
        content.pack(fill="both", expand=True, padx=30, pady=(0, 18))
        self.help_section(content, "SETUP", device.help_setup)
        self.help_section(content, "TROUBLESHOOTING", device.help_troubleshooting)
        tk.Button(dialog, text="Close", command=dialog.destroy, bg=ACCENT,
                  fg="white", activebackground="#315A82", activeforeground="white",
                  relief="flat", padx=22, pady=9).pack(anchor="e", padx=30, pady=(0, 24))

    def help_section(self, parent: tk.Widget, heading: str, lines: tuple[str, ...]) -> None:
        tk.Label(parent, text=heading, font=self.small_bold,
                 fg=MUTED, bg=SURFACE).pack(anchor="w", pady=(0, 8))
        for line in lines:
            tk.Label(parent, text="•  " + line, wraplength=545, justify="left",
                     fg=INK, bg=SURFACE).pack(anchor="w", pady=4)
        tk.Frame(parent, height=1, bg=OUTLINE).pack(fill="x", pady=16)

    @staticmethod
    def open_artifact(path: Path) -> None:
        if not path.exists():
            return
        if os.name == "nt":
            os.startfile(path)  # type: ignore[attr-defined]
        else:
            subprocess.Popen(["xdg-open", str(path)])


def main() -> None:
    App().mainloop()


if __name__ == "__main__":
    main()
