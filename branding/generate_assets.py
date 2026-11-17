"""Regenerate the Weft vector brand kit. Requires fontTools and macOS Avenir Next."""

from pathlib import Path
from fontTools.ttLib import TTFont
from fontTools.pens.svgPathPen import SVGPathPen

ROOT = Path(__file__).resolve().parent
ASSETS = ROOT / "assets"
ASSETS.mkdir(exist_ok=True)
INK, AMBER, PAPER, SLATE = "#191C23", "#F2A541", "#F5F2EB", "#A4ABB9"


def weave(warp, weft, prefix="weave", gap=2):
    """Gaps are alpha, never background-colored patches; mono works too."""
    vertical_cuts, horizontal_cuts = [], []
    for row, y in enumerate((24, 48, 72)):
        for col, x in enumerate((24, 48, 72)):
            if (row + col) % 2 == 0:
                vertical_cuts.append(f'<rect x="{x-7.5}" y="{y-7.5-gap}" width="15" height="{15+2*gap}" fill="black"/>')
            else:
                horizontal_cuts.append(f'<rect x="{x-7.5-gap}" y="{y-7.5}" width="{15+2*gap}" height="15" fill="black"/>')
    return f'''<defs>
      <mask id="{prefix}-v" maskUnits="userSpaceOnUse" x="0" y="0" width="96" height="96"><rect width="96" height="96" fill="white"/>{''.join(vertical_cuts)}</mask>
      <mask id="{prefix}-h" maskUnits="userSpaceOnUse" x="0" y="0" width="96" height="96"><rect width="96" height="96" fill="white"/>{''.join(horizontal_cuts)}</mask>
    </defs>
    <g fill="{warp}" mask="url(#{prefix}-v)">{''.join(f'<rect x="{x-7.5}" y="6" width="15" height="84" rx="7.5"/>' for x in (24,48,72))}</g>
    <g fill="{weft}" mask="url(#{prefix}-h)">{''.join(f'<rect x="6" y="{y-7.5}" width="84" height="15" rx="7.5"/>' for y in (24,48,72))}</g>'''


def svg(content, viewbox="0 0 96 96", title="Weft", size=96):
    return f'<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size if viewbox == "0 0 96 96" else 128}" viewBox="{viewbox}" role="img" aria-label="{title}"><title>{title}</title>{content}</svg>\n'


def wordmark(color):
    # Outlined lettering: no font install or network dependency for consumers.
    font = TTFont("/System/Library/Fonts/Avenir Next.ttc", fontNumber=2)
    glyphs, cmap = font.getGlyphSet(), font.getBestCmap()
    x, paths = 0, []
    scale = 0.104
    for char in "weft":
        name = cmap[ord(char)]
        pen = SVGPathPen(glyphs)
        glyphs[name].draw(pen)
        paths.append(f'<path d="{pen.getCommands()}" transform="translate({x*scale:.3f} 0) scale({scale} {-scale})"/>')
        x += font['hmtx'][name][0] - 15
    return f'<g fill="{color}" transform="translate(144 100)">{"".join(paths)}</g>'


for variant, warp, weft in (
    ("light", INK, "#C27612"),
    ("dark", SLATE, AMBER),
    ("mono-ink", INK, INK),
    ("mono-white", PAPER, PAPER),
):
    (ASSETS / f"weft-mark-{variant}.svg").write_text(svg(weave(warp, weft), title=f"Weft weave mark — {variant}"))
    lockup = f'<g transform="translate(8 16)">{weave(warp, weft)}</g>{wordmark(PAPER if variant in ("dark", "mono-white") else INK)}'
    (ASSETS / f"weft-logo-{variant}.svg").write_text(svg(lockup, "0 0 380 128", f"Weft logo — {variant}", 380))

for variant, warp, weft in (("light", "#6E7488", "#B16B0E"), ("dark", SLATE, AMBER)):
    # Tiny icons use nearly flush crossings so the two colors retain enough area.
    content = f'<g transform="scale(0.166666666667)">{weave(warp, weft, gap=0.6)}</g>'
    (ASSETS / f"weft-file-{variant}.svg").write_text(svg(content, "0 0 16 16", "Weft source file", 16).replace('height="128"', 'height="16"'))

(ASSETS / "weft-avatar.svg").write_text(svg(f'<rect width="96" height="96" rx="20" fill="{INK}"/><g transform="translate(12 12) scale(.75)">{weave(SLATE, AMBER)}</g>', title="Weft avatar"))
print(f"Wrote {len(list(ASSETS.glob('*.svg')))} SVG assets to {ASSETS}")
