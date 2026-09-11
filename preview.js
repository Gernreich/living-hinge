#!/usr/bin/env node
// Build the display rendering of one cut file in this repository.
//
//   node preview.js examples/living-hinge-wave-50x200mm.svg [OUT.svg]
//
// WHY THIS EXISTS. The eight files in previews/ shipped with no generator at
// all. .repro listed every one with a "!" -- the mark meaning "shipped without a
// generator, we know" -- and the reason was that nobody had written this. That
// mark is the right answer for a drawing made by hand. It is the wrong answer
// for a mechanical transform of a file that is itself generated, which is all
// one of these is: the cut file with its hairline thickened and painted onto a
// panel, so it reads on a page instead of vanishing against a browser's
// transparency checkerboard.
//
// Every path is copied through untouched and the result is compared with its
// source before anything is written, so a preview cannot come to show a hinge
// the sheet does not cut.
//
// NOT A CUT FILE. Cut the file in examples/, which draws the hairline a laser
// wants.
'use strict';
const fs = require('fs'), path = require('path');

const PANEL = '#c9a227';   // gold: the material that stays
const SLIT  = '#faf7f0';   // cream: what the laser removes
const STROKE = '0.6';      // mm, wide enough to read at page size

function build(src, srcRel) {
  const cutStroke = (src.match(/stroke-width="([\d.]+)"/) || [, '0.02'])[1];

  const note = `<!-- DISPLAY ONLY - not a cut file. This is ${srcRel} with the cut
     stroke thickened from ${cutStroke}mm to ${STROKE}mm and painted onto the panel so it reads
     on a page. Gold is the panel that stays, cream the slits the laser removes. A real cut
     file draws a hairline, which a browser renders almost invisibly against
     a transparency checkerboard. Cut the file in examples/, not this one. -->`;

  // The panel itself, behind everything. It goes in before the title rather than
  // after, so the first thing in the document is the ground the rest sits on.
  const vb = src.match(/viewBox="([^"]+)"/);
  if (!vb) throw new Error('no viewBox in ' + srcRel);
  const [x, y, w, h] = vb[1].trim().split(/[\s,]+/).map(Number);
  const f1 = n => n.toFixed(1);
  const bg = `  <rect id="display-bg" x="${f1(x)}" y="${f1(y)}"` +
             ` width="${f1(w)}" height="${f1(h)}" fill="${PANEL}"/>`;

  let out = src;
  out = out.replace(/(<\?xml[^>]*\?>\n)/, `$1${note}\n`);
  out = out.replace(/(<svg\b[^>]*>\n)/, `$1${bg}\n`);
  out = out.replace(/<title>([^<]*)<\/title>/, '<title>$1 — display only</title>');
  out = out.replace(/stroke="#000000"/g, `stroke="${SLIT}"`);
  out = out.replace(/stroke-width="[\d.]+"/g, `stroke-width="${STROKE}"`);
  return out;
}

const DATA = /\bd="([^"]+)"/g;

if (require.main === module) {
  const src = process.argv[2];
  if (!src) {
    console.error('usage: node preview.js CUTFILE.svg [OUT.svg]');
    console.error('  Writes previews/<name> unless an output path is given.');
    process.exit(2);
  }
  const name = path.basename(src);
  const out = process.argv[3] || path.join('previews', name);
  const text = fs.readFileSync(src, 'utf8');
  // The source path as the comment states it: repo-relative, so the note names
  // a file a reader can actually open rather than wherever this was run from.
  const rel = path.relative(process.cwd(), path.resolve(src));
  const svg = build(text, rel);

  // Geometry is not this tool's to change. Checked rather than trusted, the way
  // make-preview.py checks the same thing next door.
  const a = text.match(DATA) || [], b = svg.match(DATA) || [];
  if (a.length !== b.length || a.some((d, i) => d !== b[i])) {
    console.error('  path data changed — refusing to write');
    process.exit(1);
  }
  fs.mkdirSync(path.dirname(out), { recursive: true });
  fs.writeFileSync(out, svg);
  console.log(`  ${out}  (${(svg.length / 1024).toFixed(1)} kB)`);
}

module.exports = { build };
