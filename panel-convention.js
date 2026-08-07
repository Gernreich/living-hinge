#!/usr/bin/env node
'use strict';
// =====================================================================
// panel convention -> annotated SVG figure for the guide
// =====================================================================
//
//   node panel-convention.js [out.svg]
//
// Replaces the ASCII diagram that used to open section 1. The slits are not
// drawn by hand: this calls living-hinge-generator.js for a small straight
// panel and re-draws its actual output, so the figure cannot claim a geometry
// the tool does not produce. In particular the slits really do run out to
// x = 0 and x = width, which is the rule the section exists to state.
//
// The demo panel is deliberately short and coarse -- 60 x 84 with a 7mm pitch
// -- so the rows are legible at figure size. The proportions are illustrative;
// the shipped examples are 50 x 200.
//
const fs = require('fs');
const gen = require('./living-hinge-generator');

const OUT = process.argv[2] || 'panel-convention.svg';
const W = 60, L = 84, M = 7;                      // demo panel: width, length, margin

const r = gen.generate({ pattern: 'straight', width: W, height: L, margin: M,
                         pitch: 7, bridge: 4, targetSlit: 14 });
const slitGroup = r.svg.match(/<g id="hinge-slits"[^>]*>([\s\S]*?)<\/g>/)[1];
const slits = [...slitGroup.matchAll(/d="M([\d.-]+),([\d.-]+)\s*H([\d.-]+)"/g)]
  .map(m => ({ x1: +m[1], y: +m[2], x2: +m[3] }));
if (!slits.length) { console.error('no slits parsed'); process.exit(1); }

// ---------------------------------------------------------------- palette
const GOLD = '#c9a227', TAB = '#ab8a1e', CREAM = '#faf7f0';
const INK = '#1a1c1e', MUT = '#5b6067', RULE = '#8a8f96';

// ---------------------------------------------------------------- frame
// Frame sized to what the labels actually need: "y = length" reaches ~25 units
// left of the panel, "(rows of slits)" ~41 to its right. Generous padding just
// makes the figure mostly empty once the page scales it to the column width.
const padL = 30, padR = 46, padT = 26, padB = 36;
const x0 = -padL, y0 = -padT, w = W + padL + padR, h = L + padT + padB;
const S = 3.1;                                     // label size in panel units

const esc = s => s.replace(/&/g, '&amp;').replace(/</g, '&lt;');
const txt = (x, y, s, o = {}) =>
  `<text x="${x}" y="${y}" font-size="${o.size || S}" fill="${o.fill || INK}"` +
  `${o.anchor ? ` text-anchor="${o.anchor}"` : ''}` +
  `${o.weight ? ` font-weight="${o.weight}"` : ''}` +
  `${o.mono ? ' font-family="ui-monospace,SFMono-Regular,Menlo,monospace"' : ''}>${esc(s)}</text>`;

const parts = [];

// panel: solid tabs top and bottom, hinge field between
parts.push(`<rect x="0" y="0" width="${W}" height="${L}" fill="${GOLD}"/>`);
parts.push(`<rect x="0" y="0" width="${W}" height="${M}" fill="${TAB}"/>`);
parts.push(`<rect x="0" y="${L - M}" width="${W}" height="${M}" fill="${TAB}"/>`);

// the generator's own slits, drawn as the material they remove
parts.push(`<g stroke="${CREAM}" stroke-width="1.5" stroke-linecap="butt">` +
  slits.map(s => `<line x1="${s.x1}" y1="${s.y}" x2="${s.x2}" y2="${s.y}"/>`).join('') + `</g>`);

parts.push(`<rect x="0" y="0" width="${W}" height="${L}" fill="none" stroke="${INK}" stroke-width="0.5"/>`);
for (const y of [M, L - M])
  parts.push(`<line x1="0" y1="${y}" x2="${W}" y2="${y}" stroke="${INK}" stroke-width="0.4" stroke-dasharray="2 1.6"/>`);

// x axis, above the panel
parts.push(txt(0, -14, 'x = 0', { anchor: 'middle', mono: 1 }));
parts.push(txt(W, -14, 'x = width', { anchor: 'middle', mono: 1 }));
for (const x of [0, W])
  parts.push(`<line x1="${x}" y1="-11" x2="${x}" y2="-2" stroke="${RULE}" stroke-width="0.4"/>`);

// y axis, left of the panel
parts.push(txt(-6, 1.2, 'y = 0', { anchor: 'end', mono: 1 }));
parts.push(txt(-6, L + 1.2, 'y = length', { anchor: 'end', mono: 1 }));

// margin brackets and the field label, right of the panel
const brace = (yA, yB, label) => {
  const bx = W + 7;
  return `<path d="M${bx} ${yA}h3v${yB - yA}h-3" fill="none" stroke="${RULE}" stroke-width="0.5"/>` +
         txt(bx + 6, (yA + yB) / 2 + 1.1, label, { fill: MUT, mono: 1 });
};
parts.push(brace(0, M, 'margin'));
parts.push(brace(L - M, L, 'margin'));
parts.push(brace(M, L - M, 'hinge field'));
parts.push(txt(W + 13, L / 2 + 1.1 + S * 1.35, '(rows of slits)', { fill: MUT, mono: 1 }));

// width arrow, below
const ay = L + 15;
parts.push(`<line x1="0" y1="${ay}" x2="${W}" y2="${ay}" stroke="${RULE}" stroke-width="0.5"/>`);
parts.push(`<path d="M0 ${ay}l3.4 -1.9v3.8z M${W} ${ay}l-3.4 -1.9v3.8z" fill="${RULE}"/>`);
parts.push(`<rect x="${W / 2 - 8}" y="${ay - 2.4}" width="16" height="4.8" fill="${CREAM}"/>`);
parts.push(txt(W / 2, ay + 1.15, 'width', { anchor: 'middle', mono: 1, fill: MUT }));
parts.push(txt(W / 2, L + 27, 'slits run ACROSS the width; the panel rolls ALONG the length',
               { anchor: 'middle', fill: INK, size: S * 0.94 }));

const alt = 'The panel convention: x runs across the width, y along the length. A solid end ' +
  'tab of the margin height sits at each end of the length, and between them the hinge field ' +
  'carries staggered rows of slits that run right out to both long edges.';

fs.writeFileSync(OUT, `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="${x0} ${y0} ${w} ${h}"
     width="${w * 8}" height="${h * 8}" role="img" aria-label="${esc(alt)}">
  <title>Panel convention — width, length, margin, and the hinge field</title>
  <desc>Drawn by panel-convention.js. The slits are real output from
living-hinge-generator.js for a ${W} x ${L}mm straight panel with a ${M}mm margin, so the
figure matches what the tool makes. Proportions are illustrative; the shipped examples
are 50 x 200mm.</desc>
  <rect x="${x0}" y="${y0}" width="${w}" height="${h}" fill="${CREAM}"/>
  <g font-family="ui-sans-serif,system-ui,-apple-system,Helvetica,Arial,sans-serif">
    ${parts.join('\n    ')}
  </g>
</svg>
`);
console.log(`panel ${W} x ${L}, margin ${M}, ${slits.length} slits from the generator`);
console.log(`rows ${r.stats.rows}  pitch ${r.stats.pitch}  bridge ${r.stats.bridge}  slit ${r.stats.slit}`);
console.log(`wrote ${OUT}`);
