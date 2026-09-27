# Copyright 2026 sueuegunn
# SPDX-License-Identifier: GPL-2.0-or-later
"""Generate the notification-area / menu-bar icons.

    python tools/gen_tray_icons.py

The app icon (a dark panel with white keycaps) turns to mush at 16px on a
taskbar, so the tray needs its own mark: three keycaps in a row and nothing
else. Three variants, because the two platforms want opposite things:

    tray-on.png        solid white   - Windows, overlay showing
    tray-off.png       hollow white  - Windows, overlay hidden
    tray-template.png  solid black   - macOS, drawn as a template image so the
                                       system inverts it for the menu bar theme

Windows has no menu-bar title to put state in, so there the icon itself has to
carry it; macOS gets the layer in the title instead and only needs one icon.
"""
import os

from PIL import Image, ImageDraw

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(HERE, "src-tauri", "icons")

# Drawn at 8x and downsampled - PIL has no antialiased rounded rectangle.
SCALE = 8
SIZE = 32
KEYS = 3


def draw(fill, outline_only):
    s = SIZE * SCALE
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)

    pad = 3 * SCALE
    gap = 2 * SCALE
    key = (s - 2 * pad - (KEYS - 1) * gap) // KEYS
    top = (s - key) // 2
    for i in range(KEYS):
        x = pad + i * (key + gap)
        box = [x, top, x + key, top + key]
        if outline_only:
            d.rounded_rectangle(box, radius=key // 5, outline=fill, width=SCALE)
        else:
            d.rounded_rectangle(box, radius=key // 5, fill=fill)

    return img.resize((SIZE, SIZE), Image.LANCZOS)


def main():
    for name, fill, outline in (
        ("tray-on", (255, 255, 255, 255), False),
        ("tray-off", (255, 255, 255, 235), True),
        ("tray-template", (0, 0, 0, 255), False),
    ):
        path = os.path.join(OUT, name + ".png")
        draw(fill, outline).save(path)
        print("wrote %s" % path)


if __name__ == "__main__":
    main()
