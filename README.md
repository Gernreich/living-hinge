# Living Hinge Generator

Parametric lattice-hinge ("living hinge") SVG generator for laser cutting. Output is
millimetre-true — `1 user unit = 1 mm` with a physical `width`/`height` — so it prints
and cuts at real size.

<p>
<img src="preview-straight.svg" alt="Straight living-hinge pattern: rows of parallel slits, offset row to row, across a 60 by 90 mm panel" width="30%">
<img src="preview-wave.svg" alt="Wave living-hinge pattern: the same lattice with sinusoidal slits instead of straight ones" width="30%">
<img src="preview-honeycomb.svg" alt="Honeycomb living-hinge pattern: a hexagonal lattice that bends on both axes" width="30%">
</p>

*Three of the eight patterns — straight, wave, honeycomb — at 60 × 90 mm. Display
renderings with the cut stroke thickened; real output is a hairline.*

**[Read the operating guide](https://gernreich.github.io/living-hinge/)**

Built for **[LaserMadeMusic](https://www.youtube.com/@LaserMadeMusic)**, where the cutting
and bending are shown.

## Quick start

```
./living-hinge.js --list                          # patterns and their defaults
./living-hinge.js -p wave -w 80 -l 300            # 80 × 300 mm wave hinge
./living-hinge.js -p chevron -w 80 -l 300 -m 10   # with a 10 mm margin
./living-hinge.js --help                          # every option
```

Requires Node. No dependencies.

## Eight patterns

| Pattern | Defaults (mm) |
|---|---|
| `straight` | pitch 3, bridge 2 |
| `dogbone` | pitch 3, bridge 3, hole 0.5 |
| `wave` | pitch 3, bridge 2, wavelength 13, amplitude 1, step 0.5 |
| `chevron` | pitch 3, bridge 2, wavelength 13, amplitude 1 |
| `torsional` | pitch 3.5, bridge 2, cap 1 |
| `honeycomb` | side 5, gap 1.2 |
| `auxetic` | cell 6.25, gap 1.5 |
| `crosshatch` | cell 6.25, slit 4.5 |

## Before you cut

**Slits must reach the panel edges**, or the hinge does not flex. That constraint wins
over your other numbers: `--side` and `--cell` are fitted to the panel width, so they
are targets rather than guarantees, and the report tells you what they became.

**The tuning numbers here have not been validated against cut material.** Defaults are
reasoned, not tested. Bridge width in particular is the parameter that decides whether
a hinge flexes or snaps, and the right value depends on your stock and your kerf. The
`coupons/` directory holds a bridge sweep for exactly that purpose — six otherwise
identical 40 × 120 mm coupons varying only `--bridge`, so you can cut one sheet and
find out.

## Files

| | |
|---|---|
| `living-hinge.js` | the CLI — run it from this directory |
| `living-hinge-generator.js` | the geometry library it calls; must sit beside the CLI |
| `living-hinge-guide.md` · `.html` | operating guide; the markdown is the source |
| `NOTES.md` | how to reach this generator from a later session |
| `coupons/` | bridge-sweep test coupons and cut instructions |
| `index.html` | redirect that serves the guide on GitHub Pages |
| `preview-*.svg` | display renderings for this page — **not** cut files |

Released under [CC0 1.0](LICENSE).
