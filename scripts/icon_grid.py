#!/usr/bin/env python3
"""Folio's app icon for one small size (16-48 px), drawn on that size's
pixel grid so every edge lands on a whole pixel: the note stands straight
(a tilted note smears at this size), its outline is 1 px up to 20 px,
2 px up to 40 and 3 px above, and the tile fills the square like other
apps' icons.

    scripts/icon_grid.py 24 > icon-24.svg
"""

import sys


def svg(s: int) -> str:
    sw = 1 if s <= 20 else 2 if s <= 40 else 3  # outline width
    half = sw / 2  # a stroke centred on its path
    m = round(s * 0.16)  # margin, left and right
    top = round(s * 0.2)
    bottom = s - round(s * 0.13)
    left, right = m, s - m
    fold = max(3, round(s * 0.17))  # the turned-up corner
    gap = max(1, round(s * 0.08))
    bar_h = max(2, round(s * 0.13))  # the pink highlight
    bar_y = top + sw + gap
    inset = sw + max(1, round(s * 0.09))
    line_y = bar_y + bar_h + gap
    span = right - left - 2 * inset  # the writing's width
    line_w = round(span * 0.62)
    pin = max(3, round(s * 0.16))  # the pin's head
    pin_x = (s - pin) // 2
    pin_y = top - pin // 2
    radius = round(s * 0.22 * 2) / 2

    x0, y0, x1, y1 = left + half, top + half, right - half, bottom - half
    outline = f"M{x0} {y0} H{x1} V{y1 - fold} L{x1 - fold} {y1} H{x0} Z"
    corner = f"M{x1} {y1 - fold} H{x1 - fold} V{y1} Z"
    crease = f"M{x1} {y1 - fold} H{x1 - fold} V{y1}"  # the fold's two inner edges
    second = ""
    if s >= 28:  # room for a second, shorter line
        y2 = line_y + sw + gap
        second = f'<rect x="{left + inset}" y="{y2}" width="{round(line_w * 0.6)}" height="{sw}" fill="#111111"/>'
    # Straight edges snap to pixels; the fold's diagonal and the round
    # corners are smoothed.
    # The same light as folio-icon.svg: the tile and the paper shade from
    # top to bottom.
    return f"""<svg xmlns="http://www.w3.org/2000/svg" width="{s}" height="{s}" viewBox="0 0 {s} {s}">
  <defs>
    <linearGradient id="tile" x1="0.2" y1="0" x2="0.55" y2="1">
      <stop offset="0" stop-color="#EF5E8E"/>
      <stop offset="1" stop-color="#D9386D"/>
    </linearGradient>
    <linearGradient id="paper" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#FBF7EF"/>
      <stop offset="1" stop-color="#EDE5D4"/>
    </linearGradient>
  </defs>
  <rect width="{s}" height="{s}" rx="{radius}" fill="url(#tile)"/>
  <path d="{outline}" fill="url(#paper)"/>
  <path d="{corner}" fill="#D6CCB8"/>
  <path d="{outline} {crease}" fill="none" stroke="#111111" stroke-width="{sw}" stroke-linejoin="round"/>
  <g shape-rendering="crispEdges">
    <rect x="{left + inset}" y="{bar_y}" width="{span}" height="{bar_h}" fill="#E5487A"/>
    <rect x="{left + inset}" y="{line_y}" width="{line_w}" height="{sw}" fill="#111111"/>
    {second}
  </g>
  <rect x="{pin_x}" y="{pin_y}" width="{pin}" height="{pin}" rx="{pin / 3:.2f}" fill="#111111"/>
</svg>
"""


if __name__ == "__main__":
    print(svg(int(sys.argv[1])), end="")
