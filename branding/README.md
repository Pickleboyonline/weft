# Weft — the weave

Brand direction v1, November 2026.

**One program. Woven together.**

Weft separates application logic, placement/consistency, and platforms. Its
compiler is intended to turn those declarations into a coordinated distributed
system. A plain weave expresses that relationship: independent threads forming
one coherent structure. This direction develops the amber-and-gray motif already
present in the VS Code extension.

## Files

- `preview.html`: local, responsive brand sheet showing the actual SVG assets.
- `weft-brand-concept.png`: AI-generated visual exploration, not the vector source.
- `assets/weft-mark-{light,dark,mono-ink,mono-white}.svg`: transparent symbols.
- `assets/weft-logo-{light,dark,mono-ink,mono-white}.svg`: horizontal outlined wordmarks.
- `assets/weft-file-{light,dark}.svg`: optically adjusted 16px source-file icons.
- `assets/weft-avatar.svg`: icon on a rounded ink tile for profiles and extension listings.
- `imagegen-prompt.txt`: exact prompt used with the built-in image generation tool.
- `generate_assets.py`: deterministic vector source; requires fontTools and the
  macOS Avenir Next collection. Consumers of the exported SVGs need no fonts.

The generated concept establishes the direction. The SVG kit uses flat fills,
repeatable geometry, and real transparent underpass gaps, including in monochrome.
The existing editor extension assets and configuration have not been replaced.

## Color and typography

| Role | Color | Use |
| --- | --- | --- |
| Amber | `#F2A541` | Horizontal threads on dark backgrounds; brand accent |
| Ink | `#191C23` | Main text, light-background warp, dark surfaces |
| Slate | `#A4ABB9` | Vertical threads on dark backgrounds |
| Paper | `#F5F2EB` | Light surfaces and reversed monochrome |
| Deep amber | `#C27612` | Main symbol on light backgrounds |
| Small-icon amber | `#B16B0E` | Source-file icon on light backgrounds |

The lowercase wordmark uses outlined Avenir Next Demi Bold lettering with slightly
tighter spacing. Suggested document typography: Avenir Next with a system sans
fallback; code stays in the editor's chosen monospace. No font files are bundled.

## Usage

- Use the unboxed mark for `.weft` files; use the tile for avatars.
- Use the `light` files on light backgrounds and `dark` files on dark backgrounds.
- Use the dedicated file icons at 16px. Use the main mark at 24px or above.
- Leave at least one thread width of clear space around the visible symbol.
- Keep the mark square. Preserve the alternating over-under pattern.
- Use monochrome variants for one-color applications. Their gaps remain transparent.
- Keep the proposed tagline for brand communications. Technical docs should continue
  to describe the language as a draft; the tagline does not imply an implemented compiler.

## Research informing the direction

- [Go's brand introduction](https://go.dev/blog/go-brand) explicitly ties simple
  geometry, typography, and motion lines to the language's values of simplicity
  and efficiency.
- [Kotlin's brand assets](https://kotlinlang.org/docs/kotlin-logo.html) document
  a consistent mark/typeface relationship, protective space, and controlled use
  of colors and backgrounds.

Design inference from these examples: a credible language identity benefits from
a recognizable silhouette, a restrained palette, intentional typography, and
consistent usage. There is no single mandatory style for programming-language
logos. Weft's own textile metaphor is more relevant than copying another
language's geometry or using generic code brackets.

## Verification

All 11 SVGs parse as XML. The local preview was rendered in Chromium and visually
inspected with light/dark surfaces, monochrome variants, and 16/24/48px examples.
All preview images loaded successfully with no horizontal overflow at the
inspected desktop viewport.
