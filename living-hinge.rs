// living-hinge -- command-line front end for src/generator.rs
//
//   target/release/living-hinge --pattern wave --width 80 --length 300
//   target/release/living-hinge --all --width 2in --length 10in --units in --outdir ./hinges
//   target/release/living-hinge --list
//
// Build with `cargo build --release`. Rust since 2026-11-02 (it was living-hinge.js);
// every shipped SVG, filename and message is the same as Node's.

use living_hinge::generator::{default_of, defaults, generate, Params, Stat, COMMON, PATTERNS};
use living_hinge::jsfmt::{fixed, num, round};
use std::collections::{HashMap, HashSet};

// Length-valued options: millimetres, and a unit suffix is allowed on the input.
// Every one carries a lower bound, because zero and negative lengths are not merely bad
// hinges: a zero --step, --cell, --side or --targetSlit leaves a generator loop that never
// advances, and a zero --wavelength divides by zero and fills the path data with NaN.
// Bounds live here rather than in the generator so the error can name the flag.
// `exclusive` means the bound itself is rejected too.
struct Limit {
    key: &'static str,
    min: f64,
    exclusive: bool,
    why: &'static str,
}
const LIMITS: [Limit; 16] = [
    Limit { key: "width", min: 0.0, exclusive: true, why: "a panel needs a positive span" },
    Limit { key: "length", min: 0.0, exclusive: true, why: "a panel needs a positive length" },
    Limit { key: "margin", min: 0.0, exclusive: false, why: "a negative margin pushes cuts outside the panel outline" },
    Limit { key: "pitch", min: 0.0, exclusive: true, why: "rows advance down the panel by --pitch; zero never advances" },
    Limit { key: "bridge", min: 0.0, exclusive: true, why: "with no uncut material between collinear slits each row is one continuous cut and the panel falls apart" },
    Limit { key: "hole", min: 0.0, exclusive: false, why: "a radius is never negative (SVG rejects r < 0)" },
    Limit { key: "cap", min: 0.0, exclusive: false, why: "a cap is a half-length, never negative" },
    Limit { key: "wavelength", min: 0.0, exclusive: true, why: "a zero period divides by zero and makes every wave coordinate NaN" },
    Limit { key: "amplitude", min: 0.0, exclusive: false, why: "an amplitude is a half-height, never negative" },
    Limit { key: "side", min: 0.0, exclusive: true, why: "the hex tiling advances one side length at a time; zero never advances" },
    Limit { key: "cell", min: 0.0, exclusive: true, why: "the cell grid advances one cell at a time; zero never advances" },
    Limit { key: "gap", min: 0.0, exclusive: true, why: "the uncut node at each junction IS the hinge; a zero gap cuts it through" },
    Limit { key: "slit", min: 0.0, exclusive: true, why: "zero-length slits leave nothing but the forced edge cuts" },
    Limit { key: "step", min: 0.0, exclusive: true, why: "the wave polyline walks the slit in --step increments; zero never reaches the end" },
    Limit { key: "targetSlit", min: 0.0, exclusive: true, why: "segment count is width/targetSlit; zero asks for infinitely many segments" },
    Limit { key: "minSliver", min: 0.0, exclusive: false, why: "a negative threshold cannot drop anything" },
];

// Count-valued options: dimensionless, so no unit suffix and no fractions. `min` is
// enforced -- below 2 segments every slit spans the full width and severs the panel.
const INTEGER: [(&str, f64, &str); 3] = [
    ("segments", 2.0, "fewer than 2 segments makes every slit span the full width, cutting the panel apart"),
    ("maxCuts", 1.0, "the ceiling on emitted cuts; it stops a feature size that is small enough to exhaust memory"),
    ("maxPoints", 1.0, "the ceiling on polyline coordinates; a fine --step can exhaust this while the cut count is still modest"),
];

const PANEL: [&str; 3] = ["width", "length", "margin"]; // always forwarded, whatever the pattern
const GLOBAL: [&str; 2] = ["maxCuts", "maxPoints"]; // forwarded too, but not panel geometry
const STRING: [&str; 4] = ["pattern", "out", "outdir", "units"];
const FLAG: [&str; 5] = ["all", "help", "list", "dryRun", "force"]; // take no value

fn alias(k: &str) -> Option<&'static str> {
    Some(match k {
        "p" => "pattern",
        "w" => "width",
        "l" => "length",
        "o" => "out",
        "a" => "all",
        "h" => "help",
        "f" => "force",
        "dry-run" => "dryRun",
        "target-slit" => "targetSlit",
        "min-sliver" => "minSliver",
        "max-cuts" => "maxCuts",
        "max-points" => "maxPoints",
        _ => return None,
    })
}

/// Which tuning options actually reach a given pattern. Forwarding everything meant
/// `--hole 6 --pattern straight` tripped the generator's relief-hole check over a hole
/// straight never cuts; now inert options are reported and dropped instead.
fn applies_to(name: &str) -> Vec<&'static str> {
    let mut keys: Vec<&'static str> = defaults(name).iter().map(|(k, _)| *k).collect();
    keys.push("minSliver");
    // the row-based patterns -- the ones with a bridge -- size their slits via segmentCount
    if defaults(name).iter().any(|(k, _)| *k == "bridge") {
        keys.push("segments");
        keys.push("targetSlit");
    }
    keys
}

fn common(k: &str) -> f64 {
    COMMON.iter().find(|(c, _)| *c == k).unwrap().1
}

fn usage() {
    let lines = [
        String::new(),
        "living-hinge -- parametric lattice-hinge SVG generator".into(),
        String::new(),
        "USAGE".into(),
        "  living-hinge [--pattern <name>] [--width N] [--length N] [options]".into(),
        "  (cargo build --release puts it at target/release/living-hinge)".into(),
        String::new(),
        "PANEL".into(),
        format!("  -p, --pattern <name>   {}  (default: straight)", PATTERNS.join(" | ")),
        "  -w, --width N          span the slits run across          (default: 50)".into(),
        "  -l, --length N         direction the panel rolls          (default: 200)".into(),
        "      --margin N         solid end tab at each end          (default: 6)".into(),
        "      --units mm|in      output units; inches ONLY with --units in (default: mm)".into(),
        String::new(),
        "TUNING (pattern-dependent; anything omitted uses that pattern's default)".into(),
        "      --bridge N         uncut material between collinear slits".into(),
        "      --pitch N          row-to-row spacing".into(),
        "      --segments N       slits per row; whole number, min 2 (default scales w/ width)".into(),
        "      --hole N           dogbone relief-hole RADIUS".into(),
        "      --cap N            torsional end-cap half-length".into(),
        "      --wavelength N     wave/chevron period".into(),
        "      --amplitude N      wave/chevron half-height".into(),
        "      --step N           wave polyline resolution   (smaller = smoother, bigger file)".into(),
        "      --side N           honeycomb hex side          (target; fitted to width)".into(),
        "      --cell N           auxetic/crosshatch cell size (target; fitted to width)".into(),
        "      --gap N            honeycomb/auxetic uncut node gap".into(),
        "      --slit N           crosshatch slit length".into(),
        format!("      --targetSlit N     slit length --segments aims for  (default: {})", num(common("targetSlit"))),
        format!("      --minSliver N      drop clipped cuts shorter than this (default: {})", num(common("minSliver"))),
        format!("      --maxCuts N        ceiling on emitted cuts       (default: {})", num(common("maxCuts"))),
        format!("      --maxPoints N      ceiling on polyline points    (default: {})", num(common("maxPoints"))),
        String::new(),
        "OUTPUT".into(),
        "  -o, --out <file>       output path; not combinable with --outdir or --all".into(),
        "      --outdir <dir>     directory for generated files (default: .)".into(),
        "  -a, --all              generate every pattern at these dimensions".into(),
        "  -f, --force            overwrite existing files (default: refuse)".into(),
        "      --list             list pattern names and their defaults".into(),
        "      --dry-run          print stats without writing files".into(),
        "  -h, --help             this message".into(),
        String::new(),
        "NOTES".into(),
        "  Input values are millimetres unless suffixed (\"2in\", \"3cm\"); a suffix only".into(),
        "  converts that value. Output is always mm unless you pass --units in.".into(),
        "  --segments, --maxCuts and --maxPoints are counts: no suffix, no fractions.".into(),
        "  Tuning options that do not apply to the chosen pattern are reported and ignored.".into(),
        "  --side and --cell are targets, not guarantees: the lattice is sized so a cut lands".into(),
        "  on x=0 and x=width (an uncut rail down a long edge stops the panel flexing), so the".into(),
        "  delivered value can differ by a few percent. The stats line, the SVG <desc> and the".into(),
        "  filename all carry what was actually cut.".into(),
        "  Generated filenames carry any tuning that differs from the pattern default, so".into(),
        "  successive --bridge values land in separate files instead of one.".into(),
        "  Options that would sever the panel -- relief holes meeting across the bridge,".into(),
        "  caps or slits long enough to merge with their neighbours -- are refused, not cut.".into(),
        "  With --all, an unbuildable pattern is reported and skipped; the rest still".into(),
        "  generate and the exit status is non-zero.".into(),
        "  Defaults are tuned for ~3mm ply/acrylic and are UNVALIDATED against cut".into(),
        "  material -- cut a coupon before committing to a full panel.".into(),
        String::new(),
    ];
    println!("{}", lines.join("\n"));
}

fn list_patterns() {
    println!("\nPatterns and their defaults (mm):\n");
    for p in PATTERNS {
        let s: Vec<String> = defaults(p).iter().map(|(k, v)| format!("{k}={}", num(*v))).collect();
        println!("  {:<12}{}", p, s.join("  "));
    }
    println!(
        "\n  all patterns also use: margin={}  targetSlit={}  minSliver={}\n",
        num(common("margin")),
        num(common("targetSlit")),
        num(common("minSliver"))
    );
}

fn die(msg: &str) -> ! {
    eprintln!("living-hinge: {msg}");
    std::process::exit(1);
}

/// "12", "12.5", ".5", "2in", "30mm" -> millimetres. A unit suffix converts the input
/// only; it never changes the output units, which come from --units alone.
fn parse_len(raw: &str, flag: &str) -> f64 {
    let bad = || -> ! {
        die(&format!("bad value for --{flag}: \"{raw}\"  (expected a number, optionally suffixed mm/cm/in)"))
    };
    // ^(-?(?:\d+(?:\.\d+)?|\.\d+))\s*(mm|cm|in|")?$  -- one optional decimal point only;
    // "1.2.3" is a typo, not 1.2
    let s = raw.trim();
    let b = s.as_bytes();
    let mut i = 0;
    if i < b.len() && b[i] == b'-' {
        i += 1;
    }
    let digits = |from: usize| b[from..].iter().take_while(|c| c.is_ascii_digit()).count();
    let d = digits(i);
    if d > 0 {
        i += d;
        if i < b.len() && b[i] == b'.' && digits(i + 1) > 0 {
            i += 1 + digits(i + 1);
        }
    } else if i < b.len() && b[i] == b'.' && digits(i + 1) > 0 {
        i += 1 + digits(i + 1);
    } else {
        bad();
    }
    let number = &s[..i];
    let rest = s[i..].trim_start();
    let unit = rest.to_ascii_lowercase();
    if !matches!(unit.as_str(), "" | "mm" | "cm" | "in" | "\"") {
        bad();
    }
    let v: f64 = number.parse().unwrap_or_else(|_| bad());
    let mm = match unit.as_str() {
        "in" | "\"" => v * 25.4,
        "cm" => v * 10.0,
        _ => v,
    };

    // bound the converted value, so "-1in" is rejected the same way "-25.4" is
    if let Some(lim) = LIMITS.iter().find(|l| l.key == flag) {
        let ok = if lim.exclusive { mm > lim.min } else { mm >= lim.min };
        if !ok {
            die(&format!(
                "--{flag} must be {}{}mm (got {}mm)\n  {}",
                if lim.exclusive { "greater than " } else { "at least " },
                num(lim.min),
                num(round(mm * 10000.0) / 10000.0),
                lim.why
            ));
        }
    }
    mm
}

/// Counts are plain whole numbers -- a unit suffix here would silently scale a segment
/// count into nonsense ("2in" -> 50.8 segments).
fn parse_count(raw: &str, flag: &str) -> f64 {
    let (_, min, why) = INTEGER.iter().find(|s| s.0 == flag).unwrap();
    let s = raw.trim();
    if s.is_empty() || !s.bytes().all(|c| c.is_ascii_digit()) {
        die(&format!("--{flag} must be a whole number, got \"{raw}\""));
    }
    let v: f64 = s.parse().unwrap();
    if v < *min {
        die(&format!("--{flag} must be at least {} (got {})\n  {why}", num(*min), num(v)));
    }
    v
}

/// Filename dimension tag. Kept at 4dp so panels that genuinely differ get different
/// names -- rounding to 2dp made 50.001 and 50.004 collide on one file.
fn dim(mm: f64, units: &str) -> String {
    let v = if units == "in" { mm / 25.4 } else { mm };
    num(round(v * 10000.0) / 10000.0)
}

#[derive(Default)]
struct Opts {
    nums: HashMap<String, f64>,
    strs: HashMap<String, String>,
    flags: HashSet<String>,
}
impl Opts {
    fn n(&self, k: &str) -> Option<f64> {
        self.nums.get(k).copied()
    }
    fn s(&self, k: &str) -> Option<&str> {
        self.strs.get(k).map(|s| s.as_str())
    }
    fn f(&self, k: &str) -> bool {
        self.flags.contains(k)
    }
}

// Tuning values that differ from the pattern's own defaults become part of the filename.
// The dimension tag alone meant a run of --bridge 2, --bridge 3, --bridge 4 aimed every
// result at one path: the second run refused to overwrite, and --force quietly replaced
// the coupon you were comparing against. Default runs keep their original short names.
fn tuning_tag(name: &str, opts: &Opts, units: &str, stats: &[(String, Stat)]) -> String {
    let mut parts = Vec::new();
    // margin belongs here too: width and length are already in the dimension tag, but the
    // end-tab size changes the row count and nothing else in the name records it, so two
    // margins at one panel size used to land on a single file.
    let mut keys = vec!["margin"];
    keys.extend(applies_to(name));
    for k in keys {
        let Some(given) = opts.n(k) else { continue };
        if default_of(name, k) == Some(given) {
            continue;
        }
        // the delivered value, not the requested one -- --side and --cell get fitted to the
        // panel, and the file should be named for the geometry it actually contains
        let v = stats
            .iter()
            .find(|(s, _)| s == k)
            .and_then(|(_, v)| if let Stat::Num(x) = v { Some(*x) } else { None })
            .unwrap_or(given);
        parts.push(format!("{k}{}", if k == "segments" { num(v) } else { dim(v, units) }));
    }
    if parts.is_empty() { String::new() } else { format!("-{}", parts.join("-")) }
}

fn parse_args(argv: &[String]) -> Opts {
    let mut opts = Opts::default();
    opts.strs.insert("outdir".into(), ".".into());
    let numeric: Vec<&str> = LIMITS.iter().map(|l| l.key).collect();
    let mut i = 0;
    while i < argv.len() {
        let arg = &argv[i];
        let mut inline: Option<String> = None;
        if !arg.starts_with('-') || arg == "-" || arg == "--" {
            die(&format!("unexpected argument \"{arg}\" (options need -- prefixes)"));
        }
        let key: String;
        if !arg[1..].starts_with('-') {
            // short form: -p wave, -pwave, -w80
            let c = &arg[1..arg[1..].chars().next().map(|c| 1 + c.len_utf8()).unwrap()];
            key = match alias(c) {
                Some(k) => k.to_string(),
                None => die(&format!("unknown option \"-{c}\"  (try --help)")),
            };
            let rest = &arg[1 + c.len()..];
            if !rest.is_empty() {
                inline = Some(rest.strip_prefix('=').unwrap_or(rest).to_string());
            }
        } else {
            let mut k = arg[2..].to_string();
            if let Some(eq) = k.find('=') {
                inline = Some(k[eq + 1..].to_string());
                k.truncate(eq);
            }
            key = alias(&k).map(str::to_string).unwrap_or(k);
        }

        // Resolve the option BEFORE consuming a value, so a typo reports itself as a typo
        // rather than as "--bogus needs a value".
        let kind = if FLAG.contains(&key.as_str()) {
            "flag"
        } else if numeric.contains(&key.as_str()) {
            "length"
        } else if INTEGER.iter().any(|s| s.0 == key) {
            "count"
        } else if STRING.contains(&key.as_str()) {
            "string"
        } else {
            die(&format!("unknown option --{key}  (try --help)"))
        };

        if kind == "flag" {
            if let Some(v) = inline {
                die(&format!("--{key} takes no value (got \"{v}\")"));
            }
            if key == "help" {
                usage();
                std::process::exit(0);
            }
            if key == "list" {
                list_patterns();
                std::process::exit(0);
            }
            opts.flags.insert(key);
            i += 1;
            continue;
        }

        let val = match &inline {
            Some(v) => v.clone(),
            None => {
                i += 1;
                match argv.get(i) {
                    Some(v) => v.clone(),
                    None => die(&format!("--{key} needs a value")),
                }
            }
        };
        // don't let a bare flag swallow the following option as its value
        if inline.is_none() {
            let b = val.as_bytes();
            let after = if b.starts_with(b"--") { 2 } else if b.starts_with(b"-") { 1 } else { 0 };
            if after > 0 && b.get(after).is_some_and(|c| c.is_ascii_alphabetic()) {
                die(&format!("--{key} needs a value (got \"{val}\")"));
            }
        }
        match kind {
            "length" => {
                opts.nums.insert(key.clone(), parse_len(&val, &key));
            }
            "count" => {
                opts.nums.insert(key.clone(), parse_count(&val, &key));
            }
            _ => {
                opts.strs.insert(key.clone(), val);
            }
        }
        i += 1;
    }
    opts
}

// ---- Node's path rules, so filenames and messages read as they did ----

/// path.normalize for POSIX paths.
fn normalize(p: &str) -> String {
    if p.is_empty() {
        return ".".into();
    }
    let abs = p.starts_with('/');
    let trailing = p.ends_with('/');
    let mut out: Vec<&str> = Vec::new();
    for seg in p.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if out.last().is_some_and(|s| *s != "..") {
                    out.pop();
                } else if !abs {
                    out.push("..");
                }
            }
            s => out.push(s),
        }
    }
    let mut s = out.join("/");
    if s.is_empty() && !abs {
        s = ".".into();
    }
    if trailing && !s.is_empty() {
        s.push('/');
    }
    if abs { format!("/{s}") } else { s }
}

/// path.join(a, b).
fn join(a: &str, b: &str) -> String {
    let parts: Vec<&str> = [a, b].into_iter().filter(|s| !s.is_empty()).collect();
    normalize(&parts.join("/"))
}

/// path.dirname for POSIX paths.
fn dirname(p: &str) -> String {
    if p.is_empty() {
        return ".".into();
    }
    let t = p.trim_end_matches('/');
    if t.is_empty() {
        return "/".into();
    }
    match t.rfind('/') {
        None => ".".into(),
        Some(0) => "/".into(),
        Some(i) => {
            let d = t[..i].trim_end_matches('/');
            if d.is_empty() { "/".into() } else { d.to_string() }
        }
    }
}

fn is_dir(p: &str) -> bool {
    std::fs::metadata(p).map(|m| m.is_dir()).unwrap_or(false)
}

fn ensure_dir(dir: &str) {
    if dir.is_empty() || std::path::Path::new(dir).exists() {
        return;
    }
    if let Err(e) = std::fs::create_dir_all(dir) {
        die(&format!("cannot create output directory \"{dir}\": {e}"));
    }
    println!("created {dir}/");
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let opts = parse_args(&argv);
    let units = opts.s("units").filter(|u| !u.is_empty()).unwrap_or("mm").to_string();
    if units != "mm" && units != "in" {
        die("--units must be \"mm\" or \"in\"");
    }
    let all = opts.f("all");
    let out = opts.s("out").filter(|s| !s.is_empty()).map(str::to_string);
    let outdir = opts.s("outdir").unwrap_or(".").to_string();
    let pattern = opts.s("pattern").filter(|s| !s.is_empty());

    if all && out.is_some() {
        die("--out cannot be combined with --all (use --outdir)");
    }
    if all && pattern.is_some() {
        die("--pattern cannot be combined with --all (--all generates every pattern)");
    }
    if out.is_some() && outdir != "." {
        die("--out is a complete path; it cannot be combined with --outdir");
    }
    // "--out hinges/" used to append the extension to nothing and write a hidden ".svg"
    if let Some(o) = &out {
        if o.ends_with('/') || o.ends_with('\\') || is_dir(o) {
            die(&format!("--out names a file, but \"{o}\" is a directory (use --outdir for that)"));
        }
    }

    let targets: Vec<String> = if all {
        PATTERNS.iter().map(|s| s.to_string()).collect()
    } else {
        vec![pattern.unwrap_or("straight").to_string()]
    };
    if !all && !PATTERNS.contains(&targets[0].as_str()) {
        die(&format!("unknown pattern \"{}\"\n  expected one of: {}", targets[0], PATTERNS.join(", ")));
    }

    // Warn about tuning options that are inert for every pattern being generated. Under
    // --all a flag that suits only some patterns is applied where it fits, silently.
    let live: HashSet<&str> = targets.iter().flat_map(|n| applies_to(n)).collect();
    let checked = LIMITS.iter().map(|l| l.key).chain(INTEGER.iter().map(|s| s.0));
    for k in checked {
        if opts.n(k).is_some() && !PANEL.contains(&k) && !GLOBAL.contains(&k) && !live.contains(k) {
            eprintln!(
                "living-hinge: --{k} does not apply to {}; ignoring it",
                if all { "any pattern" } else { targets[0].as_str() }
            );
        }
    }
    // Both apply to the row patterns, so the loop above cannot see this one: an explicit
    // --segments answers the question --targetSlit was going to answer.
    if let (Some(seg), Some(ts)) = (opts.n("segments"), opts.n("targetSlit")) {
        if live.contains("segments") {
            eprintln!(
                "living-hinge: --segments {} overrides --targetSlit {}; the slit length will be whatever {} segments makes it",
                num(seg),
                num(ts),
                num(seg)
            );
        }
    }

    if !opts.f("dryRun") {
        ensure_dir(&match &out {
            Some(o) => dirname(o),
            None => outdir.clone(),
        });
    }

    // Under --all, one unbuildable pattern must not abandon the other seven; collect
    // the failures, keep generating, and report a non-zero exit at the end.
    let mut failed: Vec<String> = Vec::new();

    for name in &targets {
        let mut o = Params::default();
        for k in PANEL {
            if let Some(v) = opts.n(k) {
                o.set(if k == "length" { "height" } else { k }, v);
            }
        }
        for k in GLOBAL {
            if let Some(v) = opts.n(k) {
                o.set(k, v);
            }
        }
        for k in applies_to(name) {
            if let Some(v) = opts.n(k) {
                o.set(k, v);
            }
        }

        let mut fail = |msg: String| {
            if !all {
                die(&format!("{name}: {msg}"));
            }
            eprintln!("living-hinge: {name}: {msg}  (skipped)");
            failed.push(name.clone());
        };

        let res = match generate(name, &units, &o) {
            Ok(r) => r,
            Err(e) => {
                fail(e);
                continue;
            }
        };

        let tag = format!(
            "{}x{}{}",
            dim(res.options.get("width").unwrap(), &units),
            dim(res.options.get("height").unwrap(), &units),
            units
        );
        let mut file = match &out {
            Some(o) => o.clone(),
            None => join(
                &outdir,
                &format!("living-hinge-{name}-{tag}{}.svg", tuning_tag(name, &opts, &units, &res.stats)),
            ),
        };
        if out.is_some() {
            // /\.[^.\/\\]+$/ : an extension already there
            let has_ext = file
                .rfind('.')
                .is_some_and(|i| i + 1 < file.len() && !file[i + 1..].contains(['.', '/', '\\']));
            if !has_ext {
                file.push_str(".svg");
            }
        }

        let summary: Vec<String> =
            res.stats.iter().filter(|(k, _)| k != "pattern").map(|(k, v)| format!("{k}={v}")).collect();
        let summary = summary.join("  ");

        if opts.f("dryRun") {
            println!("{:<12}{}   -> {}  (dry run, not written)", name, summary, file);
        } else {
            // cut files get iterated on; never silently replace one the user may still need
            if !opts.f("force") && std::path::Path::new(&file).exists() {
                fail(format!("{file} already exists (use --force to overwrite)"));
                continue;
            }
            if let Err(e) = std::fs::write(&file, &res.svg) {
                fail(format!("cannot write {file}: {e}"));
                continue;
            }
            println!("{:<12}{}", name, summary);
            // KB as JavaScript counted string length: UTF-16 code units
            let units16 = res.svg.encode_utf16().count() as f64;
            println!("{:<12}-> {}  ({} KB)", "", file, fixed(units16 / 1024.0, 1));
        }
    }

    if !failed.is_empty() {
        eprintln!(
            "\nliving-hinge: {} of {} patterns failed ({})",
            failed.len(),
            targets.len(),
            failed.join(", ")
        );
        std::process::exit(1);
    }
}
