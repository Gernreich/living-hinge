// living-hinge-generator.js
//
// Parametric lattice-hinge ("living hinge") SVG generator.
// Geometry is computed in millimetres throughout; output units are chosen at emit time.
//
//   var gen = require('./living-hinge-generator');
//   gen.generate({ pattern: 'wave', width: 80, height: 300 });   // -> { svg, stats }
//
// Panel convention: `width` is the span the slits run across, `height` is the direction
// the panel rolls. Slits MUST reach x=0 and x=width or the hinge will not flex --
// a solid rail down either long edge stiffens the whole thing.

var PATTERNS = ['straight', 'dogbone', 'wave', 'chevron', 'torsional', 'honeycomb', 'auxetic', 'crosshatch'];

// Per-pattern defaults, in mm. Anything not set here falls back to COMMON.
// These are STARTING POINTS tuned for ~3mm ply/acrylic and have not been validated
// against cut material -- expect to adjust bridge and pitch after a test coupon.
// Ceilings on emitted geometry, independent of each other because they fail independently.
// A dense 50x200 panel runs a few thousand cuts, so 200k is far above any real hinge and
// far below the point where the process dies building it. Points are counted separately:
// a default wave slit is ~22 points and a deliberately fine --step 0.01 is ~1100, so a
// large panel can be well inside the cut ceiling while its polylines are not.
var COMMON = { margin: 6, targetSlit: 12, minSliver: 1.0, maxCuts: 200000, maxPoints: 3200000 };

var DEFAULTS = {
  straight:   { pitch: 3.0, bridge: 2.0 },
  dogbone:    { pitch: 3.0, bridge: 3.0, hole: 0.5 },   // bridge - 2*hole = load-bearing ligament
  wave:       { pitch: 3.0, bridge: 2.0, wavelength: 13, amplitude: 1.0, step: 0.5 },
  chevron:    { pitch: 3.0, bridge: 2.0, wavelength: 13, amplitude: 1.0 },
  torsional:  { pitch: 3.5, bridge: 2.0, cap: 1.0 },
  honeycomb:  { side: 5.0, gap: 1.2 },
  auxetic:    { cell: 6.25, gap: 1.5 },
  crosshatch: { cell: 6.25, slit: 4.5 }
};

// Lower bounds, mirrored from the CLI so a direct generate() call cannot hang either.
// The POSITIVE ones each drive a loop that advances by that value or a count derived from
// it -- at zero the loop never terminates -- or, for wavelength, a divisor.
var POSITIVE = ['width', 'height', 'pitch', 'bridge', 'wavelength', 'step',
                'side', 'cell', 'gap', 'slit', 'targetSlit'];
var NONNEG = ['margin', 'hole', 'cap', 'amplitude', 'minSliver'];

function round4(v) { return Math.round(v * 10000) / 10000; }

// ---------------------------------------------------------------- geometry core

function makeContext(o) {
  var ctx = {
    W: o.width, H: o.height,
    ymin: o.margin, ymax: o.height - o.margin,
    minSliver: o.minSliver,
    maxCuts: o.maxCuts,
    points: 0,
    dropped: 0,          // cuts discarded for coming out shorter than minSliver
    els: []
  };

  // A budget on emitted geometry, charged as the geometry is built rather than checked
  // afterwards. A plausible typo (--cell 0.05 on a 200mm panel) otherwise accumulates
  // millions of elements and kills the process with an out-of-memory FATAL ERROR that no
  // try/catch can intercept. Points are budgeted separately from elements because one
  // polyline with a tiny --step is a single element holding a million coordinates.
  // Two budgets, named separately: a fine --step can exhaust the point budget while the
  // cut count sits at a few percent of its own ceiling, and being told to raise --maxCuts
  // in that case sends you after the wrong number.
  function budgetError(what, flag, limit) {
    return new Error(what + ' budget exceeded (' + flag + ' ' + limit + ') -- the feature ' +
      'size is very small for a ' + round4(o.width) + 'x' + round4(o.height) + 'mm panel; ' +
      'coarsen it or raise ' + flag);
  }
  ctx.charge = function (n) {                       // reserve room for n coordinate pairs
    ctx.points += n;
    if (ctx.points > o.maxPoints) throw budgetError('polyline point', '--maxPoints', o.maxPoints);
  };
  ctx.emit = function (el) {
    if (ctx.els.length >= ctx.maxCuts) throw budgetError('cut', '--maxCuts', ctx.maxCuts);
    ctx.els.push(el);
  };
  // Declared before a tiling loop runs. Charging only what gets emitted is not enough:
  // a cell far smaller than minSliver drops every segment it produces, so nothing is ever
  // charged while the loop still grinds through millions of cells and exhausts memory.
  ctx.plan = function (cells) {
    if (!(cells <= ctx.maxCuts)) throw budgetError('cut', '--maxCuts', ctx.maxCuts);
  };

  // Liang-Barsky clip of a segment to the hinge region. Returns the clipped segment
  // regardless of length; the minSliver test belongs to whoever is assembling the cut,
  // since one short piece of a polyline is not a short cut.
  // A cut meant to lie exactly on a boundary does not arrive there exactly: a hex edge on
  // x=0 comes in at -2e-15, because col*hx + hx/2 does not cancel and cos(330deg) and
  // cos(390deg) differ in their last bits. That leaves the edge a hair non-parallel, so
  // Liang-Barsky reads it as crossing the boundary outside the segment and drops it whole
  // -- which is what left an uncut rail down the honeycomb edge. Snapping first makes the
  // intended-parallel case exactly parallel. 1e-9mm is far below anything cuttable.
  function snap(v, edge) { return Math.abs(v - edge) < 1e-9 ? edge : v; }

  function clipSeg(x0, y0, x1, y1) {
    x0 = snap(snap(x0, 0), ctx.W);  x1 = snap(snap(x1, 0), ctx.W);
    y0 = snap(snap(y0, ctx.ymin), ctx.ymax);  y1 = snap(snap(y1, ctx.ymin), ctx.ymax);
    var dx = x1 - x0, dy = y1 - y0, t0 = 0, t1 = 1;
    var p = [-dx, dx, -dy, dy];
    var q = [x0, ctx.W - x0, y0 - ctx.ymin, ctx.ymax - y0];
    for (var i = 0; i < 4; i++) {
      if (p[i] === 0) { if (q[i] < 0) return null; continue; }
      var r = q[i] / p[i];
      if (p[i] < 0) { if (r > t1) return null; if (r > t0) t0 = r; }
      else { if (r < t0) return null; if (r < t1) t1 = r; }
    }
    return [x0 + t0 * dx, y0 + t0 * dy, x0 + t1 * dx, y0 + t1 * dy];
  }

  ctx.clip = function (x0, y0, x1, y1) {
    var c = clipSeg(x0, y0, x1, y1);
    if (!c) return null;
    if (Math.hypot(c[2] - c[0], c[3] - c[1]) < ctx.minSliver) { ctx.dropped++; return null; }
    return c;
  };

  ctx.line = function (x0, y0, x1, y1) {
    var c = ctx.clip(x0, y0, x1, y1);
    if (c) { ctx.charge(2); ctx.emit({ t: 'line', p: c }); }
  };

  // For a cut whose whole job is to reach a long edge, minSliver is the wrong test: it
  // exists to discard useless fragments, and an edge-reaching cut is never useless however
  // short it comes out. Dropping one puts back the uncut rail it was there to prevent.
  ctx.edgeLine = function (x0, y0, x1, y1) {
    var c = clipSeg(x0, y0, x1, y1);
    if (!c || Math.hypot(c[2] - c[0], c[3] - c[1]) <= 0) return;
    ctx.charge(2);
    ctx.emit({ t: 'line', p: c });
  };

  ctx.hline = function (x0, x1, y) { ctx.charge(2); ctx.emit({ t: 'hline', p: [x0, x1, y] }); };

  // Clipped segment-by-segment: a polyline that wanders out of the hinge region comes
  // back as the pieces that stayed inside. Charging happens as the caller builds `pts`,
  // so only the element itself is emitted here.
  ctx.poly = function (pts) {
    var runs = [], cur = null, i;
    for (i = 1; i < pts.length; i++) {
      var c = clipSeg(pts[i - 1][0], pts[i - 1][1], pts[i][0], pts[i][1]);
      if (!c) { cur = null; continue; }
      var head = cur && cur[cur.length - 1];
      // An unclipped segment is returned as x0 + t*dx, and a + (b - a) === b does not hold
      // for every pair of doubles. A 1e-9mm tolerance is far below anything cuttable, and
      // without it a stray ULP would split one polyline into touching pieces.
      if (head && Math.abs(head[0] - c[0]) < 1e-9 && Math.abs(head[1] - c[1]) < 1e-9) cur.push([c[2], c[3]]);
      else { cur = [[c[0], c[1]], [c[2], c[3]]]; runs.push(cur); }
    }
    runs.forEach(function (run) {
      var len = 0;
      for (var j = 1; j < run.length; j++) len += Math.hypot(run[j][0] - run[j - 1][0], run[j][1] - run[j - 1][1]);
      if (len >= ctx.minSliver) ctx.emit({ t: 'poly', p: run });
      else ctx.dropped++;
    });
  };

  // A relief hole that does not fit inside the hinge region is dropped rather than cut
  // through the end tab. Callers check their own parameters first, so this is a backstop.
  ctx.circle = function (cx, cy, r) {
    if (cx - r < -1e-9 || cx + r > ctx.W + 1e-9 || cy - r < ctx.ymin - 1e-9 || cy + r > ctx.ymax + 1e-9) return;
    ctx.charge(1);
    ctx.emit({ t: 'circle', p: [cx, cy, r] });
  };

  return ctx;
}

// Number of full slit segments per row, so slit length stays near targetSlit as width scales
// `!== undefined` rather than a truthy test: segments:0 is a caller's mistake to report,
// not a silent request for the computed default.
function segmentCount(o) {
  if (o.segments !== undefined) return o.segments;
  return Math.max(2, Math.round(o.width / o.targetSlit));
}

// Staggered slit runs for the straight/dogbone/wave/chevron/torsional family.
// Runs are clipped to the panel, then anything shorter than minSliver is dropped --
// on narrow panels the staggered rows otherwise leave sub-millimetre fragments at the
// edges, which are useless as cuts and (in dogbone) carry a relief hole wider than the
// slit itself, off the edge of the panel.
function rowRuns(ctx, a, bridge, nSeg) {
  var period = a + bridge;
  function build(offset, lo, hi) {
    var out = [];
    for (var i = lo; i <= hi; i++) {
      var raw0 = i * period + offset, raw1 = raw0 + a;
      // The staggered row is built one segment past each end, so those two candidates
      // land wholly off the panel by design. Counting them as minSliver drops would make
      // ctx.dropped nonzero on every run and rob it of the meaning generate() reads from it.
      if (raw1 <= 0 || raw0 >= ctx.W) continue;
      var x0 = Math.max(0, raw0), x1 = Math.min(ctx.W, raw1);
      // a run that does reach the panel but comes out too short is a real dropped cut
      if (x1 - x0 >= ctx.minSliver) out.push([x0, x1]);
      else ctx.dropped++;
    }
    return out;
  }
  return { A: build(0, 0, nSeg - 1), B: build(period / 2, -1, nSeg), period: period };
}

function rowYs(ctx, pitch) {
  var span = ctx.ymax - ctx.ymin;
  ctx.plan(span / pitch);
  var n = Math.floor(span / pitch);
  // caught here, where the cause is known, rather than downstream as a bare "no cuts"
  if (n < 1) {
    throw new Error('pitch ' + pitch + 'mm leaves no room for a row in the ' + round4(span) +
      'mm between the end tabs -- lower --pitch, lengthen the panel, or lower --margin');
  }
  var y0 = ctx.ymin + (span - n * pitch) / 2 + pitch / 2;
  var ys = [];
  for (var i = 0; i < n; i++) ys.push(y0 + i * pitch);
  return ys;
}

// Shared setup for the five row-based patterns
function rowSetup(ctx, o) {
  var nSeg = segmentCount(o);
  // before rowRuns, which walks every segment: a huge count with a tiny bridge keeps `a`
  // positive, drops every run below minSliver, and so never reaches an emit to be charged
  ctx.plan(nSeg);
  var a = (ctx.W - (nSeg - 1) * o.bridge) / nSeg;
  if (a <= 0) throw new Error('bridge ' + o.bridge + 'mm is too wide for ' + nSeg + ' segments across ' + ctx.W + 'mm');
  if (o.hole !== undefined && a <= o.hole * 2) {
    throw new Error('slit ' + round4(a) + 'mm is no longer than the ' + (o.hole * 2) + 'mm relief hole -- widen the panel, lower --bridge, or lower --hole');
  }
  // The bridge is what actually holds the panel together, and a relief hole sits at each
  // end of it. Once the two holes meet, the bridge is cut through and the row parts.
  if (o.hole !== undefined && o.hole * 2 >= o.bridge) {
    throw new Error('relief holes of radius ' + o.hole + 'mm meet across a ' + o.bridge +
      'mm bridge (ligament ' + round4(o.bridge - 2 * o.hole) + 'mm) -- the row would be cut through; ' +
      'lower --hole or raise --bridge');
  }
  var ys = rowYs(ctx, o.pitch);
  // a hole is drawn around the row line, so it has to fit inside the hinge band too
  if (o.hole !== undefined && ys.length && ys[0] - o.hole < ctx.ymin - 1e-9) {
    throw new Error('relief holes of radius ' + o.hole + 'mm reach into the ' + o.margin +
      'mm end tab -- lower --hole, raise --pitch, or raise --margin');
  }
  return { nSeg: nSeg, a: a, runs: rowRuns(ctx, a, o.bridge, nSeg), ys: ys };
}

// ---------------------------------------------------------------- patterns

var BUILD = {};

BUILD.straight = function (ctx, o) {
  var s = rowSetup(ctx, o);
  s.ys.forEach(function (y, i) {
    (i % 2 ? s.runs.B : s.runs.A).forEach(function (r) { ctx.hline(r[0], r[1], y); });
  });
  return { slit: s.a, bridge: o.bridge, pitch: o.pitch, rows: s.ys.length, segments: s.nSeg };
};

BUILD.dogbone = function (ctx, o) {
  var s = rowSetup(ctx, o);
  s.ys.forEach(function (y, i) {
    (i % 2 ? s.runs.B : s.runs.A).forEach(function (r) {
      ctx.hline(r[0], r[1], y);
      // relief hole only where a slit ends inside material, not where it runs off the edge
      if (r[0] > 1e-6) ctx.circle(r[0], y, o.hole);
      if (r[1] < ctx.W - 1e-6) ctx.circle(r[1], y, o.hole);
    });
  });
  // `hole` is the radius here, the same quantity --hole takes; holeDia is what you measure
  return { slit: s.a, bridge: o.bridge, hole: o.hole, holeDia: o.hole * 2,
           ligament: o.bridge - 2 * o.hole,
           pitch: o.pitch, rows: s.ys.length, segments: s.nSeg };
};

BUILD.wave = function (ctx, o) {
  var s = rowSetup(ctx, o);
  var wy = function (x) { return o.amplitude * Math.sin(2 * Math.PI * x / o.wavelength); };
  // every row shares the sine phase, so the curves stay parallel and spacing never closes up
  s.ys.forEach(function (y, i) {
    (i % 2 ? s.runs.B : s.runs.A).forEach(function (r) {
      var pts = [], x;
      // charged per point, so a runaway --step is stopped while the array is still small
      for (x = r[0]; x < r[1] - 1e-9; x += o.step) { ctx.charge(1); pts.push([x, y + wy(x)]); }
      ctx.charge(1);
      pts.push([r[1], y + wy(r[1])]);
      ctx.poly(pts);
    });
  });
  return { slit: s.a, bridge: o.bridge, wavelength: o.wavelength, amplitude: o.amplitude,
           pitch: o.pitch, rows: s.ys.length, segments: s.nSeg };
};

BUILD.chevron = function (ctx, o) {
  var s = rowSetup(ctx, o), lam = o.wavelength;
  var tri = function (x) {
    var t = ((x / lam) % 1 + 1) % 1;
    return o.amplitude * (t < 0.25 ? 4 * t : (t < 0.75 ? 2 - 4 * t : 4 * t - 4));
  };
  s.ys.forEach(function (y, i) {
    (i % 2 ? s.runs.B : s.runs.A).forEach(function (r) {
      var xs = [r[0]];
      ctx.charge(1);
      for (var k = Math.floor(r[0] / lam) - 1; k * lam <= r[1] + lam; k++) {
        [0.25, 0.75].forEach(function (frac) {
          var bx = (k + frac) * lam;
          // charged per breakpoint, so a runaway --wavelength is stopped early
          if (bx > r[0] + 1e-9 && bx < r[1] - 1e-9) { ctx.charge(1); xs.push(bx); }
        });
      }
      ctx.charge(1);
      xs.push(r[1]);
      xs.sort(function (p, q) { return p - q; });
      ctx.poly(xs.map(function (x) { return [x, y + tri(x)]; }));
    });
  });
  return { slit: s.a, bridge: o.bridge, wavelength: lam, amplitude: o.amplitude,
           pitch: o.pitch, rows: s.ys.length, segments: s.nSeg };
};

BUILD.torsional = function (ctx, o) {
  // caps reach o.cap above and below their row; once they meet the caps of the next row
  // the panel is cut into strips down its length instead of hinged across it
  if (2 * o.cap > o.pitch) {
    throw new Error('caps of ' + o.cap + 'mm meet across a ' + o.pitch +
      'mm pitch -- the cuts would join into continuous slits; lower --cap or raise --pitch');
  }
  var s = rowSetup(ctx, o);
  s.ys.forEach(function (y, i) {
    (i % 2 ? s.runs.B : s.runs.A).forEach(function (r) {
      ctx.hline(r[0], r[1], y);
      // perpendicular caps lengthen the ligament's torsion path
      if (r[0] > 1e-6) ctx.line(r[0], y - o.cap, r[0], y + o.cap);
      if (r[1] < ctx.W - 1e-6) ctx.line(r[1], y - o.cap, r[1], y + o.cap);
    });
  });
  return { slit: s.a, bridge: o.bridge, cap: o.cap, pitch: o.pitch, rows: s.ys.length, segments: s.nSeg };
};

BUILD.honeycomb = function (ctx, o) {
  // each edge is shortened by gap/2 at both ends; at gap >= side nothing is left to cut
  // and the shortening runs past itself, drawing the edge backwards
  // Vertical hex edges recur every hx/2 across the width, so sizing the hex to make the
  // width an exact multiple of that lands an edge on x=0 and x=W instead of wherever the
  // tiling happened to stop -- otherwise a rail of uncut material survives down one long
  // edge and stiffens the panel. Crosshatch already fits its grid to the width this way.
  var m = Math.max(1, Math.round(2 * ctx.W / (Math.sqrt(3) * o.side)));
  var s = 2 * ctx.W / (Math.sqrt(3) * m);
  if (o.gap >= s) {
    throw new Error('gap ' + o.gap + 'mm is not shorter than the ' + round4(s) +
      'mm hex side fitted to a ' + round4(ctx.W) + 'mm width (from --side ' + o.side +
      ') -- no cut would be left; lower --gap or raise --side');
  }
  var hx = Math.sqrt(3) * s, vy = 1.5 * s, seen = {};
  ctx.plan((ctx.H / vy + 4) * (ctx.W / hx + 4));
  for (var row = -2; row * vy < ctx.H + 2 * vy; row++) {
    for (var col = -2; col * hx < ctx.W + 2 * hx; col++) {
      var cx = col * hx + (row % 2 ? hx / 2 : 0), cy = row * vy;
      for (var k = 0; k < 6; k++) {
        var a1 = Math.PI / 180 * (60 * k + 30), a2 = Math.PI / 180 * (60 * (k + 1) + 30);
        var p = [cx + s * Math.cos(a1), cy + s * Math.sin(a1)];
        var q = [cx + s * Math.cos(a2), cy + s * Math.sin(a2)];
        var key = [p, q].map(function (v) { return round4(v[0]) + ':' + round4(v[1]); }).sort().join('|');
        if (seen[key]) continue;
        seen[key] = 1;
        // shorten each edge so the hex vertices stay uncut -- those nodes are the hinges
        var dx = q[0] - p[0], dy = q[1] - p[1], t = (o.gap / 2) / Math.hypot(dx, dy);
        ctx.line(p[0] + dx * t, p[1] + dy * t, q[0] - dx * t, q[1] - dy * t);
      }
    }
  }
  return { side: round4(s), gap: o.gap, slit: s - o.gap, biaxial: true };
};

BUILD.auxetic = function (ctx, o) {
  // cells tile the width exactly, so the grid lands on x=0 and x=W rather than leaving
  // whatever the last partial column would leave uncut down the edge
  var nx = Math.max(1, Math.round(ctx.W / o.cell)), c = ctx.W / nx;
  if (o.gap >= c) {
    throw new Error('gap ' + o.gap + 'mm is not shorter than the ' + round4(c) +
      'mm cell fitted to a ' + round4(ctx.W) + 'mm width (from --cell ' + o.cell +
      ') -- no cut would be left; lower --gap or raise --cell');
  }
  var seen = {};
  ctx.plan((ctx.H / c + 2) * (nx + 2));
  for (var j = -1; j * c < ctx.H + c; j++) {
    for (var i = -1; i * c < ctx.W + c; i++) {
      var corners = [[i * c, j * c], [i * c + c, j * c], [i * c + c, j * c + c], [i * c, j * c + c]];
      for (var k = 0; k < 4; k++) {
        var p = corners[k], q = corners[(k + 1) % 4];
        var key = [p, q].map(function (v) { return round4(v[0]) + ':' + round4(v[1]); }).sort().join('|');
        if (seen[key]) continue;
        seen[key] = 1;
        // squares joined only at their corners -- that corner is the rotating hinge
        var dx = q[0] - p[0], dy = q[1] - p[1], t = (o.gap / 2) / Math.hypot(dx, dy);
        ctx.line(p[0] + dx * t, p[1] + dy * t, q[0] - dx * t, q[1] - dy * t);
      }
    }
  }
  return { cell: round4(c), gap: o.gap, slit: c - o.gap, biaxial: true };
};

BUILD.crosshatch = function (ctx, o) {
  var nx = Math.max(2, Math.round(ctx.W / o.cell)), c = ctx.W / nx;   // cells tile the width exactly
  // slits alternate, so the next slit in line sits 2 cells away; at that length they merge
  // into one uninterrupted cut running the length (or width) of the panel
  if (o.slit >= 2 * c) {
    throw new Error('slit ' + o.slit + 'mm spans ' + round4(2 * c) +
      'mm of alternating cells -- neighbouring slits would merge into one continuous cut; ' +
      'lower --slit or raise --cell');
  }
  var ny = Math.floor((ctx.ymax - ctx.ymin) / c);
  ctx.plan(nx * ny);
  var y0 = ctx.ymin + ((ctx.ymax - ctx.ymin) - ny * c) / 2;
  var edge = 0;
  for (var j = 0; j < ny; j++) {
    for (var i = 0; i < nx; i++) {
      var cx = (i + 0.5) * c, cy = y0 + (j + 0.5) * c;
      if ((i + j) % 2 === 0) {
        // outer columns run out to the panel edge so no uncut rail survives down either side
        var xa = (i === 0) ? 0 : cx - o.slit / 2;
        var xb = (i === nx - 1) ? ctx.W : cx + o.slit / 2;
        var atEdge = (i === 0 || i === nx - 1);
        if (atEdge) edge++;
        (atEdge ? ctx.edgeLine : ctx.line)(xa, cy, xb, cy);
      } else {
        ctx.line(cx, cy - o.slit / 2, cx, cy + o.slit / 2);
      }
    }
  }
  return { cell: round4(c), slit: o.slit, cells: nx + ' x ' + ny, edgeSlits: edge, biaxial: true };
};

// ---------------------------------------------------------------- emit

function serialize(els, k) {
  var n = function (v) { return round4(v * k); };
  return els.map(function (e) {
    var p = e.p;
    if (e.t === 'hline') return '<path d="M' + n(p[0]) + ',' + n(p[2]) + ' H' + n(p[1]) + '"/>';
    if (e.t === 'line') return '<path d="M' + n(p[0]) + ',' + n(p[1]) + ' L' + n(p[2]) + ',' + n(p[3]) + '"/>';
    if (e.t === 'circle') return '<circle cx="' + n(p[0]) + '" cy="' + n(p[1]) + '" r="' + n(p[2]) + '"/>';
    return '<path d="M' + p.map(function (q) { return n(q[0]) + ',' + n(q[1]); }).join(' L') + '"/>';
  });
}

function generate(opts) {
  var o = {}, k;
  var name = (opts && opts.pattern) || 'straight';
  if (PATTERNS.indexOf(name) < 0) throw new Error('unknown pattern "' + name + '" (expected: ' + PATTERNS.join(', ') + ')');

  var own = function (obj, key) { return Object.prototype.hasOwnProperty.call(obj, key); };
  for (k in COMMON) o[k] = COMMON[k];
  for (k in DEFAULTS[name]) o[k] = DEFAULTS[name][k];
  o.width = 50; o.height = 200; o.units = 'mm';
  // own properties only -- a caller's prototype is not a source of hinge parameters
  for (k in opts) if (own(opts, k) && opts[k] !== undefined && opts[k] !== null) o[k] = opts[k];

  // Counts, mirrored from the CLI for the same reason the length bounds are: below 2
  // segments every slit spans the full width and the panel comes apart.
  [['maxCuts', 1], ['maxPoints', 1], ['segments', 2]].forEach(function (spec) {
    var v = o[spec[0]];
    if (v === undefined) return;
    if (typeof v !== 'number' || v !== Math.floor(v) || !(v >= spec[1])) {
      throw new Error(spec[0] + ' must be a whole number >= ' + spec[1] + ' (got ' + v + ')');
    }
  });

  // written as !(v > 0) rather than v <= 0 so NaN is rejected too
  POSITIVE.forEach(function (key) {
    if (o[key] !== undefined && !(o[key] > 0)) {
      throw new Error(key + ' must be greater than 0 (got ' + o[key] + ')');
    }
  });
  NONNEG.forEach(function (key) {
    if (o[key] !== undefined && !(o[key] >= 0)) {
      throw new Error(key + ' must be 0 or greater (got ' + o[key] + ')');
    }
  });
  if (o.margin * 2 >= o.height) throw new Error('margin ' + o.margin + 'mm leaves no room in a ' + o.height + 'mm panel');
  if (o.units !== 'mm' && o.units !== 'in') throw new Error('units must be "mm" or "in"');

  var ctx = makeContext(o);
  var stats = BUILD[name](ctx, o);
  // "too small" is only one of the two ways to end up with nothing: the other is a cut
  // length set below --minSliver by the gap or slit, which has nothing to do with size
  if (!ctx.els.length) {
    throw new Error(ctx.dropped
      ? 'every cut came out shorter than --minSliver (' + o.minSliver + 'mm) and was dropped -- ' +
        'raise the cut length or lower --minSliver'
      : 'pattern produced no cuts -- panel probably too small for these parameters');
  }
  for (k in stats) if (typeof stats[k] === 'number') stats[k] = round4(stats[k]);

  // mm geometry -> output user units (1 unit = 1mm, or 1/96 in for inch output)
  var scale = o.units === 'in' ? 96 / 25.4 : 1;
  var body = serialize(ctx.els, scale);
  var vbW = round4(o.width * scale), vbH = round4(o.height * scale);
  var dimW = o.units === 'in' ? round4(o.width / 25.4) + 'in' : round4(o.width) + 'mm';
  var dimH = o.units === 'in' ? round4(o.height / 25.4) + 'in' : round4(o.height) + 'mm';
  var unitNote = o.units === 'in' ? '1 user unit = 1/96 in' : '1 user unit = 1mm';

  var params = Object.keys(stats).map(function (key) { return key + ' ' + stats[key]; }).join(', ');
  var desc = unitNote + '. Panel ' + dimW + ' x ' + dimH + ', solid end tabs ' + o.margin + 'mm. ' +
             'Pattern: ' + name + '; ' + params + '. Dimensions in mm unless noted.';

  var svg = '<?xml version="1.0" encoding="UTF-8"?>\n' +
    '<svg xmlns="http://www.w3.org/2000/svg" version="1.1"\n' +
    '     width="' + dimW + '" height="' + dimH + '" viewBox="0 0 ' + vbW + ' ' + vbH + '">\n' +
    '  <title>Living hinge - ' + name + ' - ' + dimW + ' x ' + dimH + '</title>\n' +
    '  <desc>' + desc + '</desc>\n\n' +
    '  <g fill="none" stroke="#000000" stroke-width="' + round4(0.02 * scale) + '" stroke-linecap="butt">\n\n' +
    '    <g id="outline">\n' +
    '      <rect x="0" y="0" width="' + vbW + '" height="' + vbH + '"/>\n' +
    '    </g>\n\n' +
    '    <g id="hinge-slits">\n' +
    body.map(function (e) { return '    ' + e; }).join('\n') + '\n' +
    '    </g>\n\n' +
    '  </g>\n</svg>\n';

  stats.pattern = name;
  stats.cuts = ctx.els.length;
  return { svg: svg, stats: stats, options: o };
}

module.exports = { PATTERNS: PATTERNS, DEFAULTS: DEFAULTS, COMMON: COMMON, generate: generate };
