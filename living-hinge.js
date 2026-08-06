#!/usr/bin/env node
// living-hinge.js -- command-line front end for living-hinge-generator.js
//
//   ./living-hinge.js --pattern wave --width 80 --length 300
//   ./living-hinge.js --all --width 2in --length 10in --units in --outdir ./hinges
//   ./living-hinge.js --list
//
// Requires Node >= 10.12 (String#padEnd, fs.mkdirSync recursive).

var path = require('path');
var fs = require('fs');
var gen = require(path.join(__dirname, 'living-hinge-generator'));

// Length-valued options: millimetres, and a unit suffix is allowed on the input.
// Every one carries a lower bound, because zero and negative lengths are not merely bad
// hinges: a zero --step, --cell, --side or --targetSlit leaves a generator loop that never
// advances, and a zero --wavelength divides by zero and fills the path data with NaN.
// Bounds live here rather than in the generator so the error can name the flag.
// `exclusive` means the bound itself is rejected too.
var LIMITS = {
  width:      { min: 0, exclusive: true, why: 'a panel needs a positive span' },
  length:     { min: 0, exclusive: true, why: 'a panel needs a positive length' },
  margin:     { min: 0, why: 'a negative margin pushes cuts outside the panel outline' },
  pitch:      { min: 0, exclusive: true, why: 'rows advance down the panel by --pitch; zero never advances' },
  bridge:     { min: 0, exclusive: true, why: 'with no uncut material between collinear slits each row is one continuous cut and the panel falls apart' },
  hole:       { min: 0, why: 'a radius is never negative (SVG rejects r < 0)' },
  cap:        { min: 0, why: 'a cap is a half-length, never negative' },
  wavelength: { min: 0, exclusive: true, why: 'a zero period divides by zero and makes every wave coordinate NaN' },
  amplitude:  { min: 0, why: 'an amplitude is a half-height, never negative' },
  side:       { min: 0, exclusive: true, why: 'the hex tiling advances one side length at a time; zero never advances' },
  cell:       { min: 0, exclusive: true, why: 'the cell grid advances one cell at a time; zero never advances' },
  gap:        { min: 0, exclusive: true, why: 'the uncut node at each junction IS the hinge; a zero gap cuts it through' },
  slit:       { min: 0, exclusive: true, why: 'zero-length slits leave nothing but the forced edge cuts' },
  step:       { min: 0, exclusive: true, why: 'the wave polyline walks the slit in --step increments; zero never reaches the end' },
  targetSlit: { min: 0, exclusive: true, why: 'segment count is width/targetSlit; zero asks for infinitely many segments' },
  minSliver:  { min: 0, why: 'a negative threshold cannot drop anything' }
};

// derived, so a new length option cannot be added without also giving it a bound
var NUMERIC = Object.keys(LIMITS);

// Count-valued options: dimensionless, so no unit suffix and no fractions. `min` is
// enforced -- below 2 segments every slit spans the full width and severs the panel.
var INTEGER = {
  segments:  { min: 2, why: 'fewer than 2 segments makes every slit span the full width, cutting the panel apart' },
  maxCuts:   { min: 1, why: 'the ceiling on emitted cuts; it stops a feature size that is small enough to exhaust memory' },
  maxPoints: { min: 1, why: 'the ceiling on polyline coordinates; a fine --step can exhaust this while the cut count is still modest' }
};

var PANEL = ['width', 'length', 'margin'];      // always forwarded, whatever the pattern
var GLOBAL = ['maxCuts', 'maxPoints'];          // forwarded too, but not panel geometry
var STRING = ['pattern', 'out', 'outdir', 'units'];
var FLAG = ['all', 'help', 'list', 'dryRun', 'force'];   // take no value

var ALIAS = { p: 'pattern', w: 'width', l: 'length', o: 'out', a: 'all', h: 'help', f: 'force',
              'dry-run': 'dryRun', 'target-slit': 'targetSlit', 'min-sliver': 'minSliver',
              'max-cuts': 'maxCuts', 'max-points': 'maxPoints' };

// Table lookups go through this: `--toString` and `--constructor` would otherwise find a
// function on Object.prototype and report themselves as "unknown option --function ...".
function own(obj, key) { return Object.prototype.hasOwnProperty.call(obj, key); }

// Which tuning options actually reach a given pattern. Forwarding everything meant
// `--hole 6 --pattern straight` tripped the generator's relief-hole check over a hole
// straight never cuts; now inert options are reported and dropped instead.
function appliesTo(name) {
  var keys = Object.keys(gen.DEFAULTS[name]).concat(['minSliver']);
  // the row-based patterns -- the ones with a bridge -- size their slits via segmentCount
  if (gen.DEFAULTS[name].bridge !== undefined) keys = keys.concat(['segments', 'targetSlit']);
  return keys;
}

function usage() {
  console.log([
    '',
    'living-hinge -- parametric lattice-hinge SVG generator',
    '',
    'USAGE',
    '  living-hinge.js [--pattern <name>] [--width N] [--length N] [options]',
    '',
    'PANEL',
    '  -p, --pattern <name>   ' + gen.PATTERNS.join(' | ') + '  (default: straight)',
    '  -w, --width N          span the slits run across          (default: 50)',
    '  -l, --length N         direction the panel rolls          (default: 200)',
    '      --margin N         solid end tab at each end          (default: 6)',
    '      --units mm|in      output units; inches ONLY with --units in (default: mm)',
    '',
    'TUNING (pattern-dependent; anything omitted uses that pattern\'s default)',
    '      --bridge N         uncut material between collinear slits',
    '      --pitch N          row-to-row spacing',
    '      --segments N       slits per row; whole number, min 2 (default scales w/ width)',
    '      --hole N           dogbone relief-hole RADIUS',
    '      --cap N            torsional end-cap half-length',
    '      --wavelength N     wave/chevron period',
    '      --amplitude N      wave/chevron half-height',
    '      --step N           wave polyline resolution   (smaller = smoother, bigger file)',
    '      --side N           honeycomb hex side          (target; fitted to width)',
    '      --cell N           auxetic/crosshatch cell size (target; fitted to width)',
    '      --gap N            honeycomb/auxetic uncut node gap',
    '      --slit N           crosshatch slit length',
    '      --targetSlit N     slit length --segments aims for  (default: ' + gen.COMMON.targetSlit + ')',
    '      --minSliver N      drop clipped cuts shorter than this (default: ' + gen.COMMON.minSliver + ')',
    '      --maxCuts N        ceiling on emitted cuts       (default: ' + gen.COMMON.maxCuts + ')',
    '      --maxPoints N      ceiling on polyline points    (default: ' + gen.COMMON.maxPoints + ')',
    '',
    'OUTPUT',
    '  -o, --out <file>       output path; not combinable with --outdir or --all',
    '      --outdir <dir>     directory for generated files (default: .)',
    '  -a, --all              generate every pattern at these dimensions',
    '  -f, --force            overwrite existing files (default: refuse)',
    '      --list             list pattern names and their defaults',
    '      --dry-run          print stats without writing files',
    '  -h, --help             this message',
    '',
    'NOTES',
    '  Input values are millimetres unless suffixed ("2in", "3cm"); a suffix only',
    '  converts that value. Output is always mm unless you pass --units in.',
    '  --segments, --maxCuts and --maxPoints are counts: no suffix, no fractions.',
    '  Tuning options that do not apply to the chosen pattern are reported and ignored.',
    '  --side and --cell are targets, not guarantees: the lattice is sized so a cut lands',
    '  on x=0 and x=width (an uncut rail down a long edge stops the panel flexing), so the',
    '  delivered value can differ by a few percent. The stats line, the SVG <desc> and the',
    '  filename all carry what was actually cut.',
    '  Generated filenames carry any tuning that differs from the pattern default, so',
    '  successive --bridge values land in separate files instead of one.',
    '  Options that would sever the panel -- relief holes meeting across the bridge,',
    '  caps or slits long enough to merge with their neighbours -- are refused, not cut.',
    '  With --all, an unbuildable pattern is reported and skipped; the rest still',
    '  generate and the exit status is non-zero.',
    '  Defaults are tuned for ~3mm ply/acrylic and are UNVALIDATED against cut',
    '  material -- cut a coupon before committing to a full panel.',
    ''
  ].join('\n'));
}

function listPatterns() {
  console.log('\nPatterns and their defaults (mm):\n');
  gen.PATTERNS.forEach(function (p) {
    var d = gen.DEFAULTS[p];
    var s = Object.keys(d).map(function (k) { return k + '=' + d[k]; }).join('  ');
    console.log('  ' + p.padEnd(12) + s);
  });
  console.log('\n  all patterns also use: margin=' + gen.COMMON.margin +
              '  targetSlit=' + gen.COMMON.targetSlit + '  minSliver=' + gen.COMMON.minSliver + '\n');
}

// "12", "12.5", ".5", "2in", "30mm" -> millimetres. A unit suffix converts the input
// only; it never changes the output units, which come from --units alone.
function parseLen(raw, flag) {
  // one optional decimal point only -- "1.2.3" is a typo, not 1.2
  var m = String(raw).trim().match(/^(-?(?:\d+(?:\.\d+)?|\.\d+))\s*(mm|cm|in|")?$/i);
  if (!m) die('bad value for --' + flag + ': "' + raw + '"  (expected a number, optionally suffixed mm/cm/in)');
  var v = parseFloat(m[1]), u = (m[2] || '').toLowerCase();
  var mm = (u === 'in' || u === '"') ? v * 25.4 : u === 'cm' ? v * 10 : v;

  // bound the converted value, so "-1in" is rejected the same way "-25.4" is
  var lim = own(LIMITS, flag) ? LIMITS[flag] : null;
  if (lim && (lim.exclusive ? !(mm > lim.min) : !(mm >= lim.min))) {
    die('--' + flag + ' must be ' + (lim.exclusive ? 'greater than ' : 'at least ') + lim.min +
        'mm (got ' + Math.round(mm * 10000) / 10000 + 'mm)\n  ' + lim.why);
  }
  return { mm: mm };
}

// Counts are plain whole numbers -- a unit suffix here would silently scale a
// segment count into nonsense ("2in" -> 50.8 segments).
function parseCount(raw, flag) {
  var spec = INTEGER[flag], s = String(raw).trim();
  if (!/^\d+$/.test(s)) die('--' + flag + ' must be a whole number, got "' + raw + '"');
  var v = parseInt(s, 10);
  if (v < spec.min) die('--' + flag + ' must be at least ' + spec.min + ' (got ' + v + ')\n  ' + spec.why);
  return v;
}

function die(msg) { console.error('living-hinge: ' + msg); process.exit(1); }

// Filename dimension tag. Kept at 4dp so panels that genuinely differ get different
// names -- rounding to 2dp made 50.001 and 50.004 collide on one file.
function dim(mm, units) {
  var v = units === 'in' ? mm / 25.4 : mm;
  return String(Math.round(v * 10000) / 10000);
}

// Tuning values that differ from the pattern's own defaults become part of the filename.
// The dimension tag alone meant a run of --bridge 2, --bridge 3, --bridge 4 aimed every
// result at one path: the second run refused to overwrite, and --force quietly replaced
// the coupon you were comparing against. Default runs keep their original short names.
function tuningTag(name, opts, units, stats) {
  var parts = [];
  // margin belongs here too: width and length are already in the dimension tag, but the
  // end-tab size changes the row count and nothing else in the name records it, so two
  // margins at one panel size used to land on a single file.
  ['margin'].concat(appliesTo(name)).forEach(function (k) {
    if (opts[k] === undefined) return;
    var def = own(gen.DEFAULTS[name], k) ? gen.DEFAULTS[name][k] : gen.COMMON[k];
    if (opts[k] === def) return;
    // the delivered value, not the requested one -- --side and --cell get fitted to the
    // panel width, and the file should be named for the geometry it actually contains
    var v = (stats && typeof stats[k] === 'number') ? stats[k] : opts[k];
    parts.push(k + (k === 'segments' ? v : dim(v, units)));
  });
  return parts.length ? '-' + parts.join('-') : '';
}

function isDir(p) {
  try { return fs.statSync(p).isDirectory(); } catch (e) { return false; }
}

function ensureDir(dir) {
  if (!dir || fs.existsSync(dir)) return;
  try { fs.mkdirSync(dir, { recursive: true }); }
  catch (e) { die('cannot create output directory "' + dir + '": ' + e.message); }
  console.log('created ' + dir + '/');
}

function parseArgs(argv) {
  var opts = { outdir: '.' };

  for (var i = 0; i < argv.length; i++) {
    var arg = argv[i], key, inlineVal = null;

    if (arg[0] !== '-' || arg === '-' || arg === '--') {
      die('unexpected argument "' + arg + '" (options need -- prefixes)');
    }

    if (arg[1] !== '-') {
      key = own(ALIAS, arg[1]) ? ALIAS[arg[1]] : null;   // short form: -p wave, -pwave, -w80
      if (!key) die('unknown option "-' + arg[1] + '"  (try --help)');
      if (arg.length > 2) inlineVal = arg[2] === '=' ? arg.slice(3) : arg.slice(2);
    } else {
      key = arg.slice(2);
      var eq = key.indexOf('=');
      if (eq >= 0) { inlineVal = key.slice(eq + 1); key = key.slice(0, eq); }
      if (own(ALIAS, key)) key = ALIAS[key];
    }

    // Resolve the option BEFORE consuming a value, so a typo reports itself as a typo
    // rather than as "--bogus needs a value".
    var kind = FLAG.indexOf(key) >= 0 ? 'flag'
             : NUMERIC.indexOf(key) >= 0 ? 'length'
             : own(INTEGER, key) ? 'count'
             : STRING.indexOf(key) >= 0 ? 'string' : null;
    if (!kind) die('unknown option --' + key + '  (try --help)');

    if (kind === 'flag') {
      if (inlineVal !== null) die('--' + key + ' takes no value (got "' + inlineVal + '")');
      if (key === 'help') { usage(); process.exit(0); }
      if (key === 'list') { listPatterns(); process.exit(0); }
      opts[key] = true;
      continue;
    }

    var val = inlineVal !== null ? inlineVal : argv[++i];
    if (val === undefined) die('--' + key + ' needs a value');
    // don't let a bare flag swallow the following option as its value
    if (inlineVal === null && /^--?[a-z]/i.test(val)) die('--' + key + ' needs a value (got "' + val + '")');

    if (kind === 'length') opts[key] = parseLen(val, key).mm;
    else if (kind === 'count') opts[key] = parseCount(val, key);
    else opts[key] = val;
  }

  return opts;
}

function main() {
  var opts = parseArgs(process.argv.slice(2));
  var units = opts.units || 'mm';
  if (units !== 'mm' && units !== 'in') die('--units must be "mm" or "in"');

  if (opts.all && opts.out) die('--out cannot be combined with --all (use --outdir)');
  if (opts.all && opts.pattern) die('--pattern cannot be combined with --all (--all generates every pattern)');
  if (opts.out && opts.outdir !== '.') die('--out is a complete path; it cannot be combined with --outdir');
  // "--out hinges/" used to append the extension to nothing and write a hidden ".svg"
  if (opts.out && (/[\/\\]$/.test(opts.out) || isDir(opts.out))) {
    die('--out names a file, but "' + opts.out + '" is a directory (use --outdir for that)');
  }

  var targets = opts.all ? gen.PATTERNS : [opts.pattern || 'straight'];
  if (!opts.all && gen.PATTERNS.indexOf(targets[0]) < 0) {
    die('unknown pattern "' + targets[0] + '"\n  expected one of: ' + gen.PATTERNS.join(', '));
  }

  // Warn about tuning options that are inert for every pattern being generated. Under
  // --all a flag that suits only some patterns is applied where it fits, silently.
  var live = {};
  targets.forEach(function (n) { appliesTo(n).forEach(function (k) { live[k] = 1; }); });
  NUMERIC.concat(Object.keys(INTEGER)).forEach(function (k) {
    if (opts[k] !== undefined && PANEL.indexOf(k) < 0 && GLOBAL.indexOf(k) < 0 && !live[k]) {
      console.error('living-hinge: --' + k + ' does not apply to ' +
                    (opts.all ? 'any pattern' : targets[0]) + '; ignoring it');
    }
  });
  // Both apply to the row patterns, so the loop above cannot see this one: an explicit
  // --segments answers the question --targetSlit was going to answer.
  if (opts.segments !== undefined && opts.targetSlit !== undefined && live.segments) {
    console.error('living-hinge: --segments ' + opts.segments + ' overrides --targetSlit ' +
                  opts.targetSlit + '; the slit length will be whatever ' + opts.segments +
                  ' segments makes it');
  }

  if (!opts.dryRun) ensureDir(opts.out ? path.dirname(opts.out) : opts.outdir);

  // Under --all, one unbuildable pattern must not abandon the other seven; collect
  // the failures, keep generating, and report a non-zero exit at the end.
  var failed = [];

  targets.forEach(function (name) {
    var o = { pattern: name, units: units };
    PANEL.forEach(function (k) {
      if (opts[k] !== undefined) o[k === 'length' ? 'height' : k] = opts[k];
    });
    GLOBAL.forEach(function (k) { if (opts[k] !== undefined) o[k] = opts[k]; });
    appliesTo(name).forEach(function (k) { if (opts[k] !== undefined) o[k] = opts[k]; });

    function fail(msg) {
      if (!opts.all) die(name + ': ' + msg);
      console.error('living-hinge: ' + name + ': ' + msg + '  (skipped)');
      failed.push(name);
    }

    var res;
    try { res = gen.generate(o); }
    catch (e) { return fail(e.message); }

    var tag = dim(res.options.width, units) + 'x' + dim(res.options.height, units) + units;
    var file = opts.out || path.join(opts.outdir,
      'living-hinge-' + name + '-' + tag + tuningTag(name, opts, units, res.stats) + '.svg');
    if (opts.out && !/\.[^.\/\\]+$/.test(file)) file += '.svg';

    var s = res.stats;
    var summary = Object.keys(s).filter(function (k) { return k !== 'pattern'; })
      .map(function (k) { return k + '=' + s[k]; }).join('  ');

    if (opts.dryRun) {
      console.log(name.padEnd(12) + summary + '   -> ' + file + '  (dry run, not written)');
    } else {
      // cut files get iterated on; never silently replace one the user may still need
      if (!opts.force && fs.existsSync(file)) {
        return fail(file + ' already exists (use --force to overwrite)');
      }
      try { fs.writeFileSync(file, res.svg); }
      catch (e) { return fail('cannot write ' + file + ': ' + e.message); }
      console.log(name.padEnd(12) + summary);
      console.log(''.padEnd(12) + '-> ' + file + '  (' + (res.svg.length / 1024).toFixed(1) + ' KB)');
    }
  });

  if (failed.length) {
    console.error('\nliving-hinge: ' + failed.length + ' of ' + targets.length +
                  ' patterns failed (' + failed.join(', ') + ')');
    process.exit(1);
  }
}

try { main(); }
catch (e) {
  if (process.env.LIVING_HINGE_DEBUG) throw e;
  die(((e && e.message) || String(e)) + '  (set LIVING_HINGE_DEBUG=1 for a stack trace)');
}
