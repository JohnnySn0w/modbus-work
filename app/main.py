from __future__ import annotations

import os
import shutil
import subprocess
import tkinter as tk
import zipfile
from datetime import datetime
from pathlib import Path
from tkinter import filedialog, font as tkfont, messagebox

try:
    from .catalog import DEVICES, PREMADE_CONFIGS, ROOT, DeviceDefinition
    from .discovery import PortInfo, classify_ports, discover_ports
except ImportError:  # Direct script execution
    from catalog import DEVICES, PREMADE_CONFIGS, ROOT, DeviceDefinition
    from discovery import PortInfo, classify_ports, discover_ports


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
        self.active_key: str | None = None
        self.after_id: str | None = None
        self.show_overview()

    def clear(self) -> None:
        if self.after_id:
            self.after_cancel(self.after_id)
            self.after_id = None
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
        self.update_idletasks()
        self.ports = discover_ports()
        self.classified = classify_ports(self.ports)
        self.draw_topology()
        count = len(self.ports)
        self.scan_label.configure(text=f"{count} serial interface{'s' if count != 1 else ''} found")
        self.after_id = self.after(5000, self.refresh)

    def draw_topology(self) -> None:
        self.canvas.delete("all")
        width = max(self.canvas.winfo_width(), 900)
        center_y = 185
        nodes: list[tuple[str, float]] = []
        if "bridge" in self.classified:
            nodes = [("bridge", width * 0.34), ("dpt146", width * 0.66)]
        elif "adapter" in self.classified:
            nodes = [("adapter", width * 0.38), ("dpt146", width * 0.68)]

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

        if len(nodes) == 2:
            self.canvas.create_line(nodes[0][1] + 112, center_y,
                                    nodes[1][1] - 112, center_y,
                                    fill="#AAB2BD", width=3)
            self.canvas.create_oval(width / 2 - 5, center_y - 5,
                                    width / 2 + 5, center_y + 5,
                                    fill=GOOD, outline=BG, width=3)
            self.canvas.create_text(width / 2, center_y + 23,
                                    text="RS-485", fill=MUTED, font=("Segoe UI", 9))

        for key, x in nodes:
            port = self.classified.get(key)
            connection = port.port if port else "Configured behind bridge"
            self.draw_device_card(DEVICES[key], x, center_y, connection)

    def draw_device_card(self, device: DeviceDefinition, cx: float, cy: float,
                         connection: str) -> None:
        x1, y1, x2, y2 = cx - 112, cy - 132, cx + 112, cy + 132
        items: list[int] = []
        items.append(rounded_rect(self.canvas, x1, y1, x2, y2,
                                  fill=SURFACE, outline=OUTLINE, width=1))
        items.append(self.canvas.create_oval(cx - 49, cy - 94, cx + 49, cy + 4,
                                             fill=SOFT, outline=""))
        DeviceIcon.draw(self.canvas, device.kind, cx, cy - 45, device.color)
        items.append(self.canvas.create_text(cx, cy + 28, text=device.name,
                                             font=self.title_font, fill=INK, width=195))
        items.append(self.canvas.create_text(cx, cy + 58, text=device.subtitle,
                                             fill=MUTED, width=195))
        items.append(self.canvas.create_text(cx, cy + 94, text=connection,
                                             fill=GOOD, font=self.small_bold))
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
        for index, item in enumerate(device.readouts):
            box = tk.Frame(grid, bg=SURFACE, padx=18, pady=15,
                           highlightthickness=1, highlightbackground=OUTLINE)
            box.grid(row=index // 2, column=index % 2, sticky="nsew", padx=(0, 12), pady=(0, 12))
            grid.columnconfigure(index % 2, weight=1)
            tk.Label(box, text=item.label, fg=MUTED, bg=SURFACE).pack(anchor="w")
            value = f"{item.value} {item.unit}".strip()
            tk.Label(box, text=value, fg=INK, bg=SURFACE,
                     font=("Segoe UI", 18, "bold")).pack(anchor="w", pady=(6, 2))
            tk.Label(box, text=item.quality, fg=GOOD if "validated" in item.quality.lower() else MUTED,
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

    def connection_status(self, key: str) -> str:
        if key in self.classified:
            port = self.classified[key]
            return f"Connected on {port.port} · {port.description}"
        if key == "dpt146" and "bridge" in self.classified:
            return "Configured behind the connected bridge · readouts shown are last validated"
        if key in ("hmd65", "wattnode"):
            return "Profile prepared · physical hardware not detected"
        return "Not currently detected · showing saved project information"

    def backup_configuration(self) -> None:
        stamp = datetime.now().strftime("%Y%m%d-%H%M%S")
        destination = filedialog.asksaveasfilename(
            parent=self,
            title="Save configuration backup",
            defaultextension=".zip",
            initialfile=f"enl-mod-32-backup-{stamp}.zip",
            filetypes=[("ZIP archive", "*.zip")],
        )
        if not destination:
            return
        files = [
            ROOT / "artifacts/bridge-config/vaisala-dpt146-golden.tsv",
            ROOT / "artifacts/bridge-config/vaisala-dpt146-manifest.yaml",
            ROOT / "artifacts/bridge-config/vaisala-dpt146-bridge-settings.md",
            ROOT / "artifacts/bridge-config/enl-mod-32-lorawan-status.md",
            ROOT / ".secrets/polygon-enl-mod-32-lorawan.env",
        ]
        try:
            with zipfile.ZipFile(destination, "w", zipfile.ZIP_DEFLATED) as archive:
                for path in files:
                    if path.exists():
                        archive.write(path, path.relative_to(ROOT))
            messagebox.showinfo(
                "Backup created",
                "Saved the known-good bridge configuration, settings, manifest, and recovery credentials.",
                parent=self,
            )
        except OSError as exc:
            messagebox.showerror("Backup failed", str(exc), parent=self)

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
        tk.Label(dialog, text="This copies a TSV for review; it does not write to the bridge.",
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
        tk.Button(buttons, text="Copy configuration…",
                  command=lambda: self.copy_configuration(selected.get(), dialog),
                  bg=ACCENT, fg="white", activebackground="#315A82",
                  activeforeground="white", relief="flat", padx=18,
                  pady=9, cursor="hand2").pack(side="right", padx=10)

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
