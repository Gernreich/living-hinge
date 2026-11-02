//! Parametric lattice-hinge ("living hinge") SVG generator.
//! Geometry is computed in millimetres throughout; output units are chosen at emit time.
//!
//! ```no_run
//! use living_hinge::generator::{generate, Params};
//! let r = generate("wave", "mm", &Params::from(&[("width", 80.0), ("height", 300.0)])).unwrap();
//! // r.svg, r.stats, r.options
//! ```
//!
//! Panel convention: `width` is the span the slits run across, `height` is the direction
//! the panel rolls. Slits MUST reach x=0 and x=width or the hinge will not flex --
//! a solid rail down either long edge stiffens the whole thing.
//!
//! Rust since 2026-11-02 (it was living-hinge-generator.js). Every shipped SVG redraws
//! byte-identically; numbers are written with JavaScript's rules (crate::jsfmt).
//! The trigonometry is Rust's own (the platform libm), not V8's: tested 2026-11-02 on
//! the 25 shipped files and 400 random panels, it changes nothing, because every
//! coordinate is rounded to 4 decimals long before a last-bit difference could show.

use crate::jsfmt::{max, min, num, round};
use std::collections::HashSet;
use std::f64::consts::PI;

pub const PATTERNS: [&str; 8] =
    ["straight", "dogbone", "wave", "chevron", "torsional", "honeycomb", "auxetic", "crosshatch"];

// Ceilings on emitted geometry, independent of each other because they fail independently.
// A dense 50x200 panel runs a few thousand cuts, so 200k is far above any real hinge and
// far below the point where the process dies building it. Points are counted separately:
// a default wave slit is ~22 points and a deliberately fine --step 0.01 is ~1100, so a
// large panel can be well inside the cut ceiling while its polylines are not.
pub const COMMON: [(&str, f64); 5] = [
    ("margin", 6.0),
    ("targetSlit", 12.0),
    ("minSliver", 1.0),
    ("maxCuts", 200000.0),
    ("maxPoints", 3200000.0),
];

/// Per-pattern defaults, in mm, in the order they are listed and named. Anything not set
/// here falls back to COMMON. These are STARTING POINTS tuned for ~3mm ply/acrylic and
/// have not been validated against cut material -- expect to adjust bridge and pitch
/// after a test coupon.
pub fn defaults(pattern: &str) -> &'static [(&'static str, f64)] {
    match pattern {
        "straight" => &[("pitch", 3.0), ("bridge", 2.0)],
        // bridge - 2*hole = load-bearing ligament
        "dogbone" => &[("pitch", 3.0), ("bridge", 3.0), ("hole", 0.5)],
        "wave" => &[("pitch", 3.0), ("bridge", 2.0), ("wavelength", 13.0), ("amplitude", 1.0), ("step", 0.5)],
        "chevron" => &[("pitch", 3.0), ("bridge", 2.0), ("wavelength", 13.0), ("amplitude", 1.0)],
        "torsional" => &[("pitch", 3.5), ("bridge", 2.0), ("cap", 1.0)],
        "honeycomb" => &[("side", 5.0), ("gap", 1.2)],
        "auxetic" => &[("cell", 6.25), ("gap", 1.5)],
        "crosshatch" => &[("cell", 6.25), ("slit", 4.5)],
        _ => &[],
    }
}

/// The pattern default for `key`, else the common one.
pub fn default_of(pattern: &str, key: &str) -> Option<f64> {
    defaults(pattern)
        .iter()
        .chain(COMMON.iter())
        .find(|(k, _)| *k == key)
        .map(|(_, v)| *v)
}

// Lower bounds, mirrored from the CLI so a direct generate() call cannot hang either.
// The POSITIVE ones each drive a loop that advances by that value or a count derived from
// it -- at zero the loop never terminates -- or, for wavelength, a divisor.
const POSITIVE: [&str; 11] = [
    "width", "height", "pitch", "bridge", "wavelength", "step", "side", "cell", "gap", "slit", "targetSlit",
];
const NONNEG: [&str; 5] = ["margin", "hole", "cap", "amplitude", "minSliver"];

pub fn round4(v: f64) -> f64 {
    round(v * 10000.0) / 10000.0
}

/// Named numbers, kept in the order they were set (as a JavaScript object keeps them).
#[derive(Clone, Default, Debug)]
pub struct Params(pub Vec<(String, f64)>);

impl Params {
    pub fn from(pairs: &[(&str, f64)]) -> Params {
        let mut p = Params::default();
        for (k, v) in pairs {
            p.set(k, *v);
        }
        p
    }
    pub fn get(&self, k: &str) -> Option<f64> {
        self.0.iter().find(|(key, _)| key == k).map(|(_, v)| *v)
    }
    pub fn set(&mut self, k: &str, v: f64) {
        match self.0.iter_mut().find(|(key, _)| key == k) {
            Some(e) => e.1 = v,
            None => self.0.push((k.to_string(), v)),
        }
    }
    fn n(&self, k: &str) -> f64 {
        self.get(k).unwrap_or(f64::NAN)
    }
}

/// A stat value: what the JavaScript stats object held.
#[derive(Clone, Debug)]
pub enum Stat {
    Num(f64),
    Str(String),
    Bool(bool),
}
impl std::fmt::Display for Stat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Stat::Num(v) => write!(f, "{}", num(*v)),
            Stat::Str(s) => write!(f, "{s}"),
            Stat::Bool(b) => write!(f, "{b}"),
        }
    }
}
pub type Stats = Vec<(String, Stat)>;

fn stats(pairs: Vec<(&str, Stat)>) -> Stats {
    pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}
use Stat::{Bool, Num, Str};

/// One emitted cut, in mm.
#[derive(Clone, Debug)]
pub enum El {
    HLine([f64; 3]), // x0, x1, y
    Line([f64; 4]),
    Circle([f64; 3]),
    Poly(Vec<[f64; 2]>),
}

pub struct Generated {
    pub svg: String,
    pub stats: Stats,
    pub options: Params,
    pub els: Vec<El>,
}

type R<T> = Result<T, String>;

// ---------------------------------------------------------------- geometry core

struct Ctx {
    w: f64,
    h: f64,
    ymin: f64,
    ymax: f64,
    min_sliver: f64,
    max_cuts: f64,
    max_points: f64,
    width: f64,
    height: f64,
    points: f64,
    dropped: usize, // cuts discarded for coming out shorter than minSliver
    els: Vec<El>,
}

// A cut meant to lie exactly on a boundary does not arrive there exactly: a hex edge on
// x=0 comes in at -2e-15, because col*hx + hx/2 does not cancel and cos(330deg) and
// cos(390deg) differ in their last bits. That leaves the edge a hair non-parallel, so
// Liang-Barsky reads it as crossing the boundary outside the segment and drops it whole
// -- which is what left an uncut rail down the honeycomb edge. Snapping first makes the
// intended-parallel case exactly parallel. 1e-9mm is far below anything cuttable.
fn hypot(a: f64, b: f64) -> f64 {
    a.hypot(b)
}

fn snap(v: f64, edge: f64) -> f64 {
    if (v - edge).abs() < 1e-9 { edge } else { v }
}

impl Ctx {
    // A budget on emitted geometry, charged as the geometry is built rather than checked
    // afterwards. A plausible typo (--cell 0.05 on a 200mm panel) otherwise accumulates
    // millions of elements and runs the process out of memory. Points are budgeted
    // separately from elements because one polyline with a tiny --step is a single element
    // holding a million coordinates. Two budgets, named separately: a fine --step can
    // exhaust the point budget while the cut count sits at a few percent of its own
    // ceiling, and being told to raise --maxCuts in that case sends you after the wrong
    // number.
    fn budget_error(&self, what: &str, flag: &str, limit: f64) -> String {
        format!(
            "{what} budget exceeded ({flag} {}) -- the feature size is very small for a {}x{}mm panel; coarsen it or raise {flag}",
            num(limit),
            num(round4(self.width)),
            num(round4(self.height))
        )
    }
    /// Reserve room for n coordinate pairs.
    fn charge(&mut self, n: f64) -> R<()> {
        self.points += n;
        if self.points > self.max_points {
            return Err(self.budget_error("polyline point", "--maxPoints", self.max_points));
        }
        Ok(())
    }
    fn emit(&mut self, el: El) -> R<()> {
        if self.els.len() as f64 >= self.max_cuts {
            return Err(self.budget_error("cut", "--maxCuts", self.max_cuts));
        }
        self.els.push(el);
        Ok(())
    }
    // Declared before a tiling loop runs. Charging only what gets emitted is not enough:
    // a cell far smaller than minSliver drops every segment it produces, so nothing is
    // ever charged while the loop still grinds through millions of cells.
    fn plan(&self, cells: f64) -> R<()> {
        if !(cells <= self.max_cuts) {
            return Err(self.budget_error("cut", "--maxCuts", self.max_cuts));
        }
        Ok(())
    }

    // Liang-Barsky clip of a segment to the hinge region. Returns the clipped segment
    // regardless of length; the minSliver test belongs to whoever is assembling the cut,
    // since one short piece of a polyline is not a short cut.
    fn clip_seg(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> Option<[f64; 4]> {
        let x0 = snap(snap(x0, 0.0), self.w);
        let x1 = snap(snap(x1, 0.0), self.w);
        let y0 = snap(snap(y0, self.ymin), self.ymax);
        let y1 = snap(snap(y1, self.ymin), self.ymax);
        let (dx, dy) = (x1 - x0, y1 - y0);
        let (mut t0, mut t1) = (0.0, 1.0);
        let p = [-dx, dx, -dy, dy];
        let q = [x0, self.w - x0, y0 - self.ymin, self.ymax - y0];
        for i in 0..4 {
            if p[i] == 0.0 {
                if q[i] < 0.0 {
                    return None;
                }
                continue;
            }
            let r = q[i] / p[i];
            if p[i] < 0.0 {
                if r > t1 {
                    return None;
                }
                if r > t0 {
                    t0 = r;
                }
            } else {
                if r < t0 {
                    return None;
                }
                if r < t1 {
                    t1 = r;
                }
            }
        }
        Some([x0 + t0 * dx, y0 + t0 * dy, x0 + t1 * dx, y0 + t1 * dy])
    }

    fn clip(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) -> Option<[f64; 4]> {
        let c = self.clip_seg(x0, y0, x1, y1)?;
        if hypot(c[2] - c[0], c[3] - c[1]) < self.min_sliver {
            self.dropped += 1;
            return None;
        }
        Some(c)
    }

    fn line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) -> R<()> {
        if let Some(c) = self.clip(x0, y0, x1, y1) {
            self.charge(2.0)?;
            self.emit(El::Line(c))?;
        }
        Ok(())
    }

    // For a cut whose whole job is to reach a long edge, minSliver is the wrong test: it
    // exists to discard useless fragments, and an edge-reaching cut is never useless
    // however short it comes out. Dropping one puts back the uncut rail it was there to
    // prevent.
    fn edge_line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) -> R<()> {
        let Some(c) = self.clip_seg(x0, y0, x1, y1) else { return Ok(()) };
        if hypot(c[2] - c[0], c[3] - c[1]) <= 0.0 {
            return Ok(());
        }
        self.charge(2.0)?;
        self.emit(El::Line(c))
    }

    fn hline(&mut self, x0: f64, x1: f64, y: f64) -> R<()> {
        self.charge(2.0)?;
        self.emit(El::HLine([x0, x1, y]))
    }

    // Clipped segment-by-segment: a polyline that wanders out of the hinge region comes
    // back as the pieces that stayed inside. Charging happens as the caller builds `pts`,
    // so only the element itself is emitted here.
    fn poly(&mut self, pts: &[[f64; 2]]) -> R<()> {
        let mut runs: Vec<Vec<[f64; 2]>> = Vec::new();
        let mut open = false; // whether the last run is still being extended
        for i in 1..pts.len() {
            let Some(c) = self.clip_seg(pts[i - 1][0], pts[i - 1][1], pts[i][0], pts[i][1]) else {
                open = false;
                continue;
            };
            // An unclipped segment is returned as x0 + t*dx, and a + (b - a) == b does not
            // hold for every pair of doubles. A 1e-9mm tolerance is far below anything
            // cuttable, and without it a stray ULP would split one polyline into touching
            // pieces.
            let joins = open && {
                let head = runs.last().unwrap().last().unwrap();
                (head[0] - c[0]).abs() < 1e-9 && (head[1] - c[1]).abs() < 1e-9
            };
            if joins {
                runs.last_mut().unwrap().push([c[2], c[3]]);
            } else {
                runs.push(vec![[c[0], c[1]], [c[2], c[3]]]);
                open = true;
            }
        }
        for run in runs {
            let mut len = 0.0;
            for j in 1..run.len() {
                len += hypot(run[j][0] - run[j - 1][0], run[j][1] - run[j - 1][1]);
            }
            if len >= self.min_sliver {
                self.emit(El::Poly(run))?;
            } else {
                self.dropped += 1;
            }
        }
        Ok(())
    }

    // A relief hole that does not fit inside the hinge region is dropped rather than cut
    // through the end tab. Callers check their own parameters first, so this is a backstop.
    fn circle(&mut self, cx: f64, cy: f64, r: f64) -> R<()> {
        if cx - r < -1e-9 || cx + r > self.w + 1e-9 || cy - r < self.ymin - 1e-9 || cy + r > self.ymax + 1e-9 {
            return Ok(());
        }
        self.charge(1.0)?;
        self.emit(El::Circle([cx, cy, r]))
    }
}

/// Number of full slit segments per row, so slit length stays near targetSlit as width
/// scales.
fn segment_count(o: &Params) -> f64 {
    match o.get("segments") {
        Some(s) => s,
        None => max(2.0, round(o.n("width") / o.n("targetSlit"))),
    }
}

struct Runs {
    a: Vec<[f64; 2]>,
    b: Vec<[f64; 2]>,
}

// Staggered slit runs for the straight/dogbone/wave/chevron/torsional family.
// Runs are clipped to the panel, then anything shorter than minSliver is dropped --
// on narrow panels the staggered rows otherwise leave sub-millimetre fragments at the
// edges, which are useless as cuts and (in dogbone) carry a relief hole wider than the
// slit itself, off the edge of the panel.
fn row_runs(ctx: &mut Ctx, a: f64, bridge: f64, n_seg: f64) -> Runs {
    let period = a + bridge;
    let mut build = |offset: f64, lo: f64, hi: f64| {
        let mut out = Vec::new();
        let mut i = lo;
        while i <= hi {
            let raw0 = i * period + offset;
            let raw1 = raw0 + a;
            i += 1.0;
            // The staggered row is built one segment past each end, so those two candidates
            // land wholly off the panel by design. Counting them as minSliver drops would
            // make ctx.dropped nonzero on every run and rob it of the meaning generate()
            // reads from it.
            if raw1 <= 0.0 || raw0 >= ctx.w {
                continue;
            }
            let (x0, x1) = (max(0.0, raw0), min(ctx.w, raw1));
            // a run that does reach the panel but comes out too short is a real dropped cut
            if x1 - x0 >= ctx.min_sliver {
                out.push([x0, x1]);
            } else {
                ctx.dropped += 1;
            }
        }
        out
    };
    let a_runs = build(0.0, 0.0, n_seg - 1.0);
    let b_runs = build(period / 2.0, -1.0, n_seg);
    Runs { a: a_runs, b: b_runs }
}

fn row_ys(ctx: &Ctx, pitch: f64) -> R<Vec<f64>> {
    let span = ctx.ymax - ctx.ymin;
    ctx.plan(span / pitch)?;
    let n = (span / pitch).floor();
    // caught here, where the cause is known, rather than downstream as a bare "no cuts"
    if n < 1.0 {
        return Err(format!(
            "pitch {}mm leaves no room for a row in the {}mm between the end tabs -- lower --pitch, lengthen the panel, or lower --margin",
            num(pitch),
            num(round4(span))
        ));
    }
    let y0 = ctx.ymin + (span - n * pitch) / 2.0 + pitch / 2.0;
    let mut ys = Vec::new();
    let mut i = 0.0;
    while i < n {
        ys.push(y0 + i * pitch);
        i += 1.0;
    }
    Ok(ys)
}

struct RowSetup {
    n_seg: f64,
    a: f64,
    runs: Runs,
    ys: Vec<f64>,
}
impl RowSetup {
    /// Runs for row i: A on even rows, B (staggered) on odd.
    fn row(&self, i: usize) -> Vec<[f64; 2]> {
        if i % 2 == 1 { self.runs.b.clone() } else { self.runs.a.clone() }
    }
}

// Shared setup for the five row-based patterns
fn row_setup(ctx: &mut Ctx, o: &Params) -> R<RowSetup> {
    let n_seg = segment_count(o);
    // before rowRuns, which walks every segment: a huge count with a tiny bridge keeps `a`
    // positive, drops every run below minSliver, and so never reaches an emit to be charged
    ctx.plan(n_seg)?;
    let bridge = o.n("bridge");
    let a = (ctx.w - (n_seg - 1.0) * bridge) / n_seg;
    if a <= 0.0 {
        return Err(format!(
            "bridge {}mm is too wide for {} segments across {}mm",
            num(bridge),
            num(n_seg),
            num(ctx.w)
        ));
    }
    let hole = o.get("hole");
    if let Some(hole) = hole {
        if a <= hole * 2.0 {
            return Err(format!(
                "slit {}mm is no longer than the {}mm relief hole -- widen the panel, lower --bridge, or lower --hole",
                num(round4(a)),
                num(hole * 2.0)
            ));
        }
        // The bridge is what actually holds the panel together, and a relief hole sits at
        // each end of it. Once the two holes meet, the bridge is cut through and the row
        // parts.
        if hole * 2.0 >= bridge {
            return Err(format!(
                "relief holes of radius {}mm meet across a {}mm bridge (ligament {}mm) -- the row would be cut through; lower --hole or raise --bridge",
                num(hole),
                num(bridge),
                num(round4(bridge - 2.0 * hole))
            ));
        }
    }
    let ys = row_ys(ctx, o.n("pitch"))?;
    // a hole is drawn around the row line, so it has to fit inside the hinge band too
    if let Some(hole) = hole {
        if !ys.is_empty() && ys[0] - hole < ctx.ymin - 1e-9 {
            return Err(format!(
                "relief holes of radius {}mm reach into the {}mm end tab -- lower --hole, raise --pitch, or raise --margin",
                num(hole),
                num(o.n("margin"))
            ));
        }
    }
    let runs = row_runs(ctx, a, bridge, n_seg);
    Ok(RowSetup { n_seg, a, runs, ys })
}

// ---------------------------------------------------------------- patterns

fn build_straight(ctx: &mut Ctx, o: &Params) -> R<Stats> {
    let s = row_setup(ctx, o)?;
    for (i, &y) in s.ys.iter().enumerate() {
        for r in s.row(i) {
            ctx.hline(r[0], r[1], y)?;
        }
    }
    Ok(stats(vec![
        ("slit", Num(s.a)),
        ("bridge", Num(o.n("bridge"))),
        ("pitch", Num(o.n("pitch"))),
        ("rows", Num(s.ys.len() as f64)),
        ("segments", Num(s.n_seg)),
    ]))
}

fn build_dogbone(ctx: &mut Ctx, o: &Params) -> R<Stats> {
    let s = row_setup(ctx, o)?;
    let hole = o.n("hole");
    for (i, &y) in s.ys.iter().enumerate() {
        for r in s.row(i) {
            ctx.hline(r[0], r[1], y)?;
            // relief hole only where a slit ends inside material, not where it runs off the edge
            if r[0] > 1e-6 {
                ctx.circle(r[0], y, hole)?;
            }
            if r[1] < ctx.w - 1e-6 {
                ctx.circle(r[1], y, hole)?;
            }
        }
    }
    // `hole` is the radius here, the same quantity --hole takes; holeDia is what you measure
    Ok(stats(vec![
        ("slit", Num(s.a)),
        ("bridge", Num(o.n("bridge"))),
        ("hole", Num(hole)),
        ("holeDia", Num(hole * 2.0)),
        ("ligament", Num(o.n("bridge") - 2.0 * hole)),
        ("pitch", Num(o.n("pitch"))),
        ("rows", Num(s.ys.len() as f64)),
        ("segments", Num(s.n_seg)),
    ]))
}

fn build_wave(ctx: &mut Ctx, o: &Params) -> R<Stats> {
    let s = row_setup(ctx, o)?;
    let (amp, wl, step) = (o.n("amplitude"), o.n("wavelength"), o.n("step"));
    let wy = |x: f64| amp * (2.0 * PI * x / wl).sin();
    // every row shares the sine phase, so the curves stay parallel and spacing never closes up
    for (i, &y) in s.ys.iter().enumerate() {
        for r in s.row(i) {
            let mut pts = Vec::new();
            let mut x = r[0];
            // charged per point, so a runaway --step is stopped while the array is still small
            while x < r[1] - 1e-9 {
                ctx.charge(1.0)?;
                pts.push([x, y + wy(x)]);
                x += step;
            }
            ctx.charge(1.0)?;
            pts.push([r[1], y + wy(r[1])]);
            ctx.poly(&pts)?;
        }
    }
    Ok(stats(vec![
        ("slit", Num(s.a)),
        ("bridge", Num(o.n("bridge"))),
        ("wavelength", Num(wl)),
        ("amplitude", Num(amp)),
        ("pitch", Num(o.n("pitch"))),
        ("rows", Num(s.ys.len() as f64)),
        ("segments", Num(s.n_seg)),
    ]))
}

fn build_chevron(ctx: &mut Ctx, o: &Params) -> R<Stats> {
    let s = row_setup(ctx, o)?;
    let (lam, amp) = (o.n("wavelength"), o.n("amplitude"));
    let tri = |x: f64| {
        let t = ((x / lam) % 1.0 + 1.0) % 1.0;
        amp * (if t < 0.25 { 4.0 * t } else if t < 0.75 { 2.0 - 4.0 * t } else { 4.0 * t - 4.0 })
    };
    for (i, &y) in s.ys.iter().enumerate() {
        for r in s.row(i) {
            let mut xs = vec![r[0]];
            ctx.charge(1.0)?;
            let mut k = (r[0] / lam).floor() - 1.0;
            while k * lam <= r[1] + lam {
                for frac in [0.25, 0.75] {
                    let bx = (k + frac) * lam;
                    // charged per breakpoint, so a runaway --wavelength is stopped early
                    if bx > r[0] + 1e-9 && bx < r[1] - 1e-9 {
                        ctx.charge(1.0)?;
                        xs.push(bx);
                    }
                }
                k += 1.0;
            }
            ctx.charge(1.0)?;
            xs.push(r[1]);
            xs.sort_by(|p, q| p.partial_cmp(q).unwrap_or(std::cmp::Ordering::Equal));
            let pts: Vec<[f64; 2]> = xs.iter().map(|&x| [x, y + tri(x)]).collect();
            ctx.poly(&pts)?;
        }
    }
    Ok(stats(vec![
        ("slit", Num(s.a)),
        ("bridge", Num(o.n("bridge"))),
        ("wavelength", Num(lam)),
        ("amplitude", Num(amp)),
        ("pitch", Num(o.n("pitch"))),
        ("rows", Num(s.ys.len() as f64)),
        ("segments", Num(s.n_seg)),
    ]))
}

fn build_torsional(ctx: &mut Ctx, o: &Params) -> R<Stats> {
    let (cap, pitch) = (o.n("cap"), o.n("pitch"));
    // caps reach o.cap above and below their row; once they meet the caps of the next row
    // the panel is cut into strips down its length instead of hinged across it
    if 2.0 * cap > pitch {
        return Err(format!(
            "caps of {}mm meet across a {}mm pitch -- the cuts would join into continuous slits; lower --cap or raise --pitch",
            num(cap),
            num(pitch)
        ));
    }
    let s = row_setup(ctx, o)?;
    for (i, &y) in s.ys.iter().enumerate() {
        for r in s.row(i) {
            ctx.hline(r[0], r[1], y)?;
            // perpendicular caps lengthen the ligament's torsion path
            if r[0] > 1e-6 {
                ctx.line(r[0], y - cap, r[0], y + cap)?;
            }
            if r[1] < ctx.w - 1e-6 {
                ctx.line(r[1], y - cap, r[1], y + cap)?;
            }
        }
    }
    Ok(stats(vec![
        ("slit", Num(s.a)),
        ("bridge", Num(o.n("bridge"))),
        ("cap", Num(cap)),
        ("pitch", Num(pitch)),
        ("rows", Num(s.ys.len() as f64)),
        ("segments", Num(s.n_seg)),
    ]))
}

/// The dedup key of an edge: both ends to 4dp, sorted, so an edge shared by two cells is
/// cut once.
fn edge_key(p: [f64; 2], q: [f64; 2]) -> String {
    let mut ks = [p, q].map(|v| format!("{}:{}", num(round4(v[0])), num(round4(v[1]))));
    ks.sort();
    ks.join("|")
}

fn build_honeycomb(ctx: &mut Ctx, o: &Params) -> R<Stats> {
    // each edge is shortened by gap/2 at both ends; at gap >= side nothing is left to cut
    // and the shortening runs past itself, drawing the edge backwards.
    // Vertical hex edges recur every hx/2 across the width, so sizing the hex to make the
    // width an exact multiple of that lands an edge on x=0 and x=W instead of wherever the
    // tiling happened to stop -- otherwise a rail of uncut material survives down one long
    // edge and stiffens the panel. Crosshatch already fits its grid to the width this way.
    let (side, gap) = (o.n("side"), o.n("gap"));
    let m = max(1.0, round(2.0 * ctx.w / (3f64.sqrt() * side)));
    let s = 2.0 * ctx.w / (3f64.sqrt() * m);
    if gap >= s {
        return Err(format!(
            "gap {}mm is not shorter than the {}mm hex side fitted to a {}mm width (from --side {}) -- no cut would be left; lower --gap or raise --side",
            num(gap),
            num(round4(s)),
            num(round4(ctx.w)),
            num(side)
        ));
    }
    let (hx, vy) = (3f64.sqrt() * s, 1.5 * s);
    let mut seen: HashSet<String> = HashSet::new();
    ctx.plan((ctx.h / vy + 4.0) * (ctx.w / hx + 4.0))?;
    let mut row = -2i64;
    while (row as f64) * vy < ctx.h + 2.0 * vy {
        let mut col = -2i64;
        while (col as f64) * hx < ctx.w + 2.0 * hx {
            let cx = col as f64 * hx + if row % 2 != 0 { hx / 2.0 } else { 0.0 };
            let cy = row as f64 * vy;
            for k in 0..6 {
                let k = k as f64;
                let a1 = PI / 180.0 * (60.0 * k + 30.0);
                let a2 = PI / 180.0 * (60.0 * (k + 1.0) + 30.0);
                let p = [cx + s * a1.cos(), cy + s * a1.sin()];
                let q = [cx + s * a2.cos(), cy + s * a2.sin()];
                if !seen.insert(edge_key(p, q)) {
                    continue;
                }
                // shorten each edge so the hex vertices stay uncut -- those nodes are the hinges
                let (dx, dy) = (q[0] - p[0], q[1] - p[1]);
                let t = (gap / 2.0) / hypot(dx, dy);
                ctx.line(p[0] + dx * t, p[1] + dy * t, q[0] - dx * t, q[1] - dy * t)?;
            }
            col += 1;
        }
        row += 1;
    }
    Ok(stats(vec![
        ("side", Num(round4(s))),
        ("gap", Num(gap)),
        ("slit", Num(s - gap)),
        ("biaxial", Bool(true)),
    ]))
}

fn build_auxetic(ctx: &mut Ctx, o: &Params) -> R<Stats> {
    // cells tile the width exactly, so the grid lands on x=0 and x=W rather than leaving
    // whatever the last partial column would leave uncut down the edge
    let (cell, gap) = (o.n("cell"), o.n("gap"));
    let nx = max(1.0, round(ctx.w / cell));
    let c = ctx.w / nx;
    if gap >= c {
        return Err(format!(
            "gap {}mm is not shorter than the {}mm cell fitted to a {}mm width (from --cell {}) -- no cut would be left; lower --gap or raise --cell",
            num(gap),
            num(round4(c)),
            num(round4(ctx.w)),
            num(cell)
        ));
    }
    let mut seen: HashSet<String> = HashSet::new();
    ctx.plan((ctx.h / c + 2.0) * (nx + 2.0))?;
    let mut j = -1.0f64;
    while j * c < ctx.h + c {
        let mut i = -1.0f64;
        while i * c < ctx.w + c {
            let corners = [[i * c, j * c], [i * c + c, j * c], [i * c + c, j * c + c], [i * c, j * c + c]];
            for k in 0..4 {
                let (p, q) = (corners[k], corners[(k + 1) % 4]);
                if !seen.insert(edge_key(p, q)) {
                    continue;
                }
                // squares joined only at their corners -- that corner is the rotating hinge
                let (dx, dy) = (q[0] - p[0], q[1] - p[1]);
                let t = (gap / 2.0) / hypot(dx, dy);
                ctx.line(p[0] + dx * t, p[1] + dy * t, q[0] - dx * t, q[1] - dy * t)?;
            }
            i += 1.0;
        }
        j += 1.0;
    }
    Ok(stats(vec![
        ("cell", Num(round4(c))),
        ("gap", Num(gap)),
        ("slit", Num(c - gap)),
        ("biaxial", Bool(true)),
    ]))
}

fn build_crosshatch(ctx: &mut Ctx, o: &Params) -> R<Stats> {
    let (cell, slit) = (o.n("cell"), o.n("slit"));
    let nx = max(2.0, round(ctx.w / cell)); // cells tile the width exactly
    let c = ctx.w / nx;
    // slits alternate, so the next slit in line sits 2 cells away; at that length they merge
    // into one uninterrupted cut running the length (or width) of the panel
    if slit >= 2.0 * c {
        return Err(format!(
            "slit {}mm spans {}mm of alternating cells -- neighbouring slits would merge into one continuous cut; lower --slit or raise --cell",
            num(slit),
            num(round4(2.0 * c))
        ));
    }
    let ny = ((ctx.ymax - ctx.ymin) / c).floor();
    ctx.plan(nx * ny)?;
    let y0 = ctx.ymin + ((ctx.ymax - ctx.ymin) - ny * c) / 2.0;
    let mut edge = 0usize;
    let mut j = 0.0f64;
    while j < ny {
        let mut i = 0.0f64;
        while i < nx {
            let cx = (i + 0.5) * c;
            let cy = y0 + (j + 0.5) * c;
            if (i + j) % 2.0 == 0.0 {
                // outer columns run out to the panel edge so no uncut rail survives down either side
                let xa = if i == 0.0 { 0.0 } else { cx - slit / 2.0 };
                let xb = if i == nx - 1.0 { ctx.w } else { cx + slit / 2.0 };
                let at_edge = i == 0.0 || i == nx - 1.0;
                if at_edge {
                    edge += 1;
                    ctx.edge_line(xa, cy, xb, cy)?;
                } else {
                    ctx.line(xa, cy, xb, cy)?;
                }
            } else {
                ctx.line(cx, cy - slit / 2.0, cx, cy + slit / 2.0)?;
            }
            i += 1.0;
        }
        j += 1.0;
    }
    Ok(stats(vec![
        ("cell", Num(round4(c))),
        ("slit", Num(slit)),
        ("cells", Str(format!("{} x {}", num(nx), num(ny)))),
        ("edgeSlits", Num(edge as f64)),
        ("biaxial", Bool(true)),
    ]))
}

// ---------------------------------------------------------------- emit

fn serialize(els: &[El], k: f64) -> Vec<String> {
    let n = |v: f64| num(round4(v * k));
    els.iter()
        .map(|e| match e {
            El::HLine(p) => format!("<path d=\"M{},{} H{}\"/>", n(p[0]), n(p[2]), n(p[1])),
            El::Line(p) => format!("<path d=\"M{},{} L{},{}\"/>", n(p[0]), n(p[1]), n(p[2]), n(p[3])),
            El::Circle(p) => format!("<circle cx=\"{}\" cy=\"{}\" r=\"{}\"/>", n(p[0]), n(p[1]), n(p[2])),
            El::Poly(p) => {
                let pts: Vec<String> = p.iter().map(|q| format!("{},{}", n(q[0]), n(q[1]))).collect();
                format!("<path d=\"M{}\"/>", pts.join(" L"))
            }
        })
        .collect()
}

/// Build one hinge. `opts` overrides the pattern's defaults; `units` is "mm" or "in" and
/// changes only what is written out.
pub fn generate(pattern: &str, units: &str, opts: &Params) -> R<Generated> {
    let name = if pattern.is_empty() { "straight" } else { pattern };
    if !PATTERNS.contains(&name) {
        return Err(format!("unknown pattern \"{name}\" (expected: {})", PATTERNS.join(", ")));
    }
    let mut o = Params::default();
    for (k, v) in COMMON.iter().chain(defaults(name).iter()) {
        o.set(k, *v);
    }
    o.set("width", 50.0);
    o.set("height", 200.0);
    for (k, v) in &opts.0 {
        o.set(k, *v);
    }

    // Counts, mirrored from the CLI for the same reason the length bounds are: below 2
    // segments every slit spans the full width and the panel comes apart.
    for (k, lo) in [("maxCuts", 1.0), ("maxPoints", 1.0), ("segments", 2.0)] {
        if let Some(v) = o.get(k) {
            if v != v.floor() || !(v >= lo) {
                return Err(format!("{k} must be a whole number >= {} (got {})", num(lo), num(v)));
            }
        }
    }
    // written as !(v > 0) rather than v <= 0 so NaN is rejected too
    for k in POSITIVE {
        if let Some(v) = o.get(k) {
            if !(v > 0.0) {
                return Err(format!("{k} must be greater than 0 (got {})", num(v)));
            }
        }
    }
    for k in NONNEG {
        if let Some(v) = o.get(k) {
            if !(v >= 0.0) {
                return Err(format!("{k} must be 0 or greater (got {})", num(v)));
            }
        }
    }
    let (width, height, margin) = (o.n("width"), o.n("height"), o.n("margin"));
    if margin * 2.0 >= height {
        return Err(format!("margin {}mm leaves no room in a {}mm panel", num(margin), num(height)));
    }
    if units != "mm" && units != "in" {
        return Err("units must be \"mm\" or \"in\"".into());
    }

    let mut ctx = Ctx {
        w: width,
        h: height,
        ymin: margin,
        ymax: height - margin,
        min_sliver: o.n("minSliver"),
        max_cuts: o.n("maxCuts"),
        max_points: o.n("maxPoints"),
        width,
        height,
        points: 0.0,
        dropped: 0,
        els: Vec::new(),
    };
    let built = match name {
        "straight" => build_straight(&mut ctx, &o),
        "dogbone" => build_dogbone(&mut ctx, &o),
        "wave" => build_wave(&mut ctx, &o),
        "chevron" => build_chevron(&mut ctx, &o),
        "torsional" => build_torsional(&mut ctx, &o),
        "honeycomb" => build_honeycomb(&mut ctx, &o),
        "auxetic" => build_auxetic(&mut ctx, &o),
        _ => build_crosshatch(&mut ctx, &o),
    };
    let mut stats = built?;
    // "too small" is only one of the two ways to end up with nothing: the other is a cut
    // length set below --minSliver by the gap or slit, which has nothing to do with size
    if ctx.els.is_empty() {
        return Err(if ctx.dropped > 0 {
            format!(
                "every cut came out shorter than --minSliver ({}mm) and was dropped -- raise the cut length or lower --minSliver",
                num(o.n("minSliver"))
            )
        } else {
            "pattern produced no cuts -- panel probably too small for these parameters".into()
        });
    }
    for (_, v) in stats.iter_mut() {
        if let Num(x) = v {
            *x = round4(*x);
        }
    }

    // mm geometry -> output user units (1 unit = 1mm, or 1/96 in for inch output)
    let inch = units == "in";
    let scale = if inch { 96.0 / 25.4 } else { 1.0 };
    let body = serialize(&ctx.els, scale);
    let (vb_w, vb_h) = (num(round4(width * scale)), num(round4(height * scale)));
    let dim = |v: f64| {
        if inch { format!("{}in", num(round4(v / 25.4))) } else { format!("{}mm", num(round4(v))) }
    };
    let (dim_w, dim_h) = (dim(width), dim(height));
    let unit_note = if inch { "1 user unit = 1/96 in" } else { "1 user unit = 1mm" };

    let params: Vec<String> = stats.iter().map(|(k, v)| format!("{k} {v}")).collect();
    let desc = format!(
        "{unit_note}. Panel {dim_w} x {dim_h}, solid end tabs {}mm. Pattern: {name}; {}. Dimensions in mm unless noted.",
        num(margin),
        params.join(", ")
    );
    let lines: Vec<String> = body.iter().map(|e| format!("    {e}")).collect();
    let svg = format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" version="1.1"
     width="{dim_w}" height="{dim_h}" viewBox="0 0 {vb_w} {vb_h}">
  <title>Living hinge - {name} - {dim_w} x {dim_h}</title>
  <desc>{desc}</desc>

  <g fill="none" stroke="#000000" stroke-width="{sw}" stroke-linecap="butt">

    <g id="outline">
      <rect x="0" y="0" width="{vb_w}" height="{vb_h}"/>
    </g>

    <g id="hinge-slits">
{body}
    </g>

  </g>
</svg>
"##,
        sw = num(round4(0.02 * scale)),
        body = lines.join("\n"),
    );

    stats.push(("pattern".into(), Str(name.into())));
    stats.push(("cuts".into(), Num(ctx.els.len() as f64)));
    Ok(Generated { svg, stats, options: o, els: ctx.els })
}
