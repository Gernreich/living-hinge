# Living Hinge Generator

**[← All projects](https://gernreich.github.io/)**

<!-- readme-only -->
**[Read the writeup](https://gernreich.github.io/living-hinge/)**

<!-- page-only **[Read the README](https://github.com/Gernreich/living-hinge)** -->

Parametric lattice-hinge ("living hinge") SVG generator for laser cutting. Output is
millimetre-true — `1 user unit = 1mm` with a physical `width`/`height` — so it prints
and cuts at real size.

<table>
<tr>
<td align="center"><a href="examples/living-hinge-straight-50x200mm.svg"><img src="previews/living-hinge-straight-50x200mm.svg" alt="straight pattern: rows of parallel slits, offset row to row" width="125"></a></td>
<td align="center"><a href="examples/living-hinge-dogbone-50x200mm.svg"><img src="previews/living-hinge-dogbone-50x200mm.svg" alt="dogbone pattern: straight slits with a round relief hole at each end" width="125"></a></td>
<td align="center"><a href="examples/living-hinge-wave-50x200mm.svg"><img src="previews/living-hinge-wave-50x200mm.svg" alt="wave pattern: sinusoidal slits" width="125"></a></td>
<td align="center"><a href="examples/living-hinge-chevron-50x200mm.svg"><img src="previews/living-hinge-chevron-50x200mm.svg" alt="chevron pattern: zigzag slits" width="125"></a></td>
</tr>
<tr>
<td align="center"><sub>straight</sub></td>
<td align="center"><sub>dogbone</sub></td>
<td align="center"><sub>wave</sub></td>
<td align="center"><sub>chevron</sub></td>
</tr>
<tr>
<td align="center"><a href="examples/living-hinge-torsional-50x200mm.svg"><img src="previews/living-hinge-torsional-50x200mm.svg" alt="torsional pattern: I-cuts, straight slits with perpendicular end caps" width="125"></a></td>
<td align="center"><a href="examples/living-hinge-honeycomb-50x200mm.svg"><img src="previews/living-hinge-honeycomb-50x200mm.svg" alt="honeycomb pattern: a hexagonal lattice, bends on both axes" width="125"></a></td>
<td align="center"><a href="examples/living-hinge-auxetic-50x200mm.svg"><img src="previews/living-hinge-auxetic-50x200mm.svg" alt="auxetic pattern: rotating squares, bends on both axes" width="125"></a></td>
<td align="center"><a href="examples/living-hinge-crosshatch-50x200mm.svg"><img src="previews/living-hinge-crosshatch-50x200mm.svg" alt="crosshatch pattern: alternating short slits, bends on both axes" width="125"></a></td>
</tr>
<tr>
<td align="center"><sub>torsional</sub></td>
<td align="center"><sub>honeycomb</sub></td>
<td align="center"><sub>auxetic</sub></td>
<td align="center"><sub>crosshatch</sub></td>
</tr>
</table>

*All eight patterns at 50 × 200mm. Click one to download the cut file. The pictures are
display renderings — **gold is the panel that stays, cream the slits the laser removes** —
with the stroke thickened and painted onto the panel. A real cut file draws a hairline on
nothing at all, which a browser shows almost invisibly against a transparency
checkerboard.*

Built for **[LaserMadeMusic](https://www.youtube.com/@LaserMadeMusic)**, so an instrument
cut from flat sheet can have curved surfaces; the cutting and bending are shown there.

**[The rest of the build files](https://gernreich.github.io/lasermademusic.html)** — every instrument,
generator and tool, indexed.

**[The operating guide](living-hinge-guide.html)** — every option, the library API, and
what each pattern is for.

**[Repository](https://github.com/Gernreich/living-hinge)** — the code, if you want to
change a pattern or read how it works.

**[Download everything as a ZIP](https://github.com/Gernreich/living-hinge/archive/refs/heads/main.zip)** — the source, the guide, all eight examples and the coupons.

## Quick start

```
cargo build --release                             # once; puts the programs in target/release/
target/release/living-hinge --list                          # patterns and their defaults
target/release/living-hinge -p wave -w 80 -l 300            # 80 × 300mm wave hinge
target/release/living-hinge -p chevron -w 80 -l 300 --margin 10   # with a 10mm margin
target/release/living-hinge --help                          # every option
```

Every pattern is committed cut-ready in [`examples/`](examples) at 50 × 200mm, so you can
cut one without running anything. Those are the files the guide's per-pattern figures
describe.

Requires Rust ([rustup](https://rustup.rs)). No dependencies beyond the standard library.
The tools were JavaScript until 2026-11-02; the Rust port redraws every shipped file
byte for byte.

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

**The stock is 3mm Baltic birch plywood**, and the defaults are aimed at it. **The tuning
numbers are still reasoned rather than tested**, though: bridge width decides whether a
hinge flexes or snaps, and the right value depends on your kerf as well as your stock. The
`coupons/` directory holds a bridge sweep for exactly that purpose — six otherwise
identical 40 × 120mm coupons varying only `--bridge`, so you can cut one sheet and
find out.

## Files

| | |
|---|---|
| `living-hinge.rs` | the command-line program, built as `target/release/living-hinge` |
| `src/generator.rs` | the geometry library it calls |
| `src/jsfmt.rs` | writes numbers exactly as JavaScript did, so the files stay byte-identical |
| `src/lib.rs` · `Cargo.toml` · `Cargo.lock` | the Rust crate around them |
| [`living-hinge-guide.md`](living-hinge-guide.md) · [`.html`](living-hinge-guide.html) | operating guide; the markdown is the source |
| `NOTES.md` | how to reach this generator from a later session |
| `preview.rs` | draws `previews/` from the cut files: the hairline thickened onto a gold panel |
| `panel-convention.rs` | draws the guide's [`panel-convention.svg`](panel-convention.svg) from real generator output |
| `examples/` | one cut-ready file per pattern, all eight, at 50 × 200mm |
| `coupons/` | bridge-sweep test coupons and cut instructions |
| `index.html` | this page, generated from `README.md`; GitHub Pages serves it |
| `previews/` | display renderings of every pattern — **not** cut files |

Released under [CC0 1.0](LICENSE).

## Credits

Made by Michael Gernreich with [Claude](https://www.anthropic.com/claude) (Anthropic), working in Claude Code, which co-wrote the code and the documentation. Commits Claude made carry a `Co-Authored-By: Claude` line.
