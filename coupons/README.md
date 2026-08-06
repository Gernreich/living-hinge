# Bridge sweep coupons — straight pattern, 3 mm stock

Six 40 × 120 mm coupons that vary **only** `--bridge`, the uncut ligament between collinear
slits. Everything else is held constant: pitch 3, margin 6, 3 segments, 36 rows, 126 cuts each.

Generated with:

```bash
cd ~/LaserMadeMusic/GIT/living-hinge
for b in 1.0 1.25 1.5 1.75 2.0 2.5; do
  ./living-hinge.js -w 40 -l 120 --bridge $b --outdir ./coupons
done
```

## What to cut

**`coupon-sheet-bridge-sweep-290x132mm.svg`** — all six on one 290 × 132 mm sheet, one job.

Two groups:

| Group | Do this |
|---|---|
| `#cuts` | **Cut.** Hairline 0.02 mm stroke, 756 paths |
| `#labels` | **Engrave**, or delete it. Fill-only text, no stroke |

If your cutter substitutes or drops the label font, the coupons are still identifiable:
**bridge increases left to right**, 1.0 → 2.5 mm.

The six individual files are also here if you'd rather cut them separately or re-nest them.

## Coupons

| # | Bridge (nominal) | Slit | Ligament after 0.15 mm kerf | File |
|---|---|---|---|---|
| 1 | 1.00 mm | 12.6667 | ~0.85 mm | `...-bridge1.svg` |
| 2 | 1.25 mm | 12.5 | ~1.10 mm | `...-bridge1.25.svg` |
| 3 | 1.50 mm | 12.3333 | ~1.35 mm | `...-bridge1.5.svg` |
| 4 | 1.75 mm | 12.1667 | ~1.60 mm | `...-bridge1.75.svg` |
| 5 | 2.00 mm | 12.0 | ~1.85 mm | `...-40x120mm.svg` *(the default — plain name)* |
| 6 | 2.50 mm | 11.6667 | ~2.35 mm | `...-bridge2.5.svg` |

`--bridge` is a **centreline** value. The beam removes material on both sides, so the real
ligament is roughly `bridge − kerf`. Measure your kerf once (cut a 20 mm square, measure it,
the shortfall is one kerf) and the column above shifts accordingly.

## Procedure

Cut in the **grain orientation you will actually build in** — ply behaves differently with
slits across the grain versus along it. If you're unsure which you'll use, cut two sheets.

1. **Bend each to 90°.** Note which snap immediately.
2. **Cycle the survivors 20–30 times.** Fatigue kills living hinges in service, not the
   first bend.
3. **Bend one to destruction** to see how much margin you actually have.
4. Take the narrowest bridge that survives cycling and **add 0.25 mm** as your working value.

## Results

| Bridge | Survived first 90°? | Cycles before failure | Failure mode | Notes |
|---|---|---|---|---|
| 1.00 | | | | |
| 1.25 | | | | |
| 1.50 | | | | |
| 1.75 | | | | |
| 2.00 | | | | |
| 2.50 | | | | |

Material: ________________  Thickness: ______  Grain: along / across
Machine: ________________  Power/speed: ______  Measured kerf: ______ mm

**Failure modes worth distinguishing:** snapped at the ligament mid-span (bridge too narrow)
versus cracked at a slit end (stress riser — switch to the `dogbone` pattern rather than
widening the bridge).

## When you have numbers

Report them and they'll be written into `DEFAULTS` in `living-hinge-generator.js` and into
the project memory, so future work starts from measurements instead of estimates. Something
like *"1.0 and 1.25 snapped on the first bend, 1.5 held 30 cycles, 3 mm birch ply, slits
across the grain"* is enough.

See `../living-hinge-guide.md` §10 for the wider context.
