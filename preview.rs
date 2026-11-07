// Build the display rendering of one cut file in this repository.
//
//   target/release/preview examples/living-hinge-wave-50x200mm.svg [OUT.svg]
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
//
// Rust since 2026-11-02 (it was preview.js); every preview is unchanged.

use living_hinge::jsfmt::fixed;
use std::path::{Component, Path, PathBuf};

const PANEL: &str = "#c9a227"; // gold: the material that stays
const SLIT: &str = "#faf7f0"; // cream: what the laser removes
const STROKE: &str = "0.6"; // mm, wide enough to read at page size

/// Every `stroke-width="<digits and dots>"`: (start, end) of the value.
fn stroke_widths(s: &str) -> Vec<(usize, usize)> {
    let key = "stroke-width=\"";
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = s[from..].find(key) {
        let v = from + i + key.len();
        let n = s[v..].bytes().take_while(|c| c.is_ascii_digit() || *c == b'.').count();
        if n > 0 && s[v + n..].starts_with('"') {
            out.push((v, v + n));
        }
        from = v;
    }
    out
}

/// Every `d="..."` attribute value whose `d` starts a word.
fn path_data(s: &str) -> Vec<&str> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = s[from..].find("d=\"") {
        let at = from + i;
        from = at + 3;
        let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
        if at > 0 && word(b[at - 1]) {
            continue;
        }
        if let Some(end) = s[from..].find('"') {
            if end > 0 {
                out.push(&s[from..from + end]);
            }
        }
    }
    out
}

/// Insert `add` after the first match of `open ... >\n` (the tag's `>` straight
/// followed by a newline), as `.replace(/(<open[^>]*>\n)/, "$1" + add)` does.
fn insert_after_tag(s: &str, open: &str, close: &str, add: &str) -> String {
    let mut from = 0;
    while let Some(i) = s[from..].find(open) {
        let at = from + i;
        from = at + 1;
        let after = at + open.len();
        // <svg\b : the next character must not continue the word
        if open == "<svg" && s[after..].bytes().next().is_some_and(|c| c.is_ascii_alphanumeric() || c == b'_') {
            continue;
        }
        let Some(j) = s[after..].find('>') else { continue };
        let gt = after + j;
        if !s[..=gt].ends_with(close) || !s[gt + 1..].starts_with('\n') {
            continue;
        }
        return format!("{}{add}\n{}", &s[..gt + 2], &s[gt + 2..]);
    }
    s.to_string()
}

fn build(src: &str, src_rel: &str) -> Result<String, String> {
    let cut_stroke = stroke_widths(src).first().map(|&(a, b)| &src[a..b]).unwrap_or("0.02").to_string();

    let note = format!(
        "<!-- DISPLAY ONLY - not a cut file. This is {src_rel} with the cut
     stroke thickened from {cut_stroke}mm to {STROKE}mm and painted onto the panel so it reads
     on a page. Gold is the panel that stays, cream the slits the laser removes. A real cut
     file draws a hairline, which a browser renders almost invisibly against
     a transparency checkerboard. Cut the file in examples/, not this one. -->"
    );

    // The panel itself, behind everything. It goes in before the title rather than
    // after, so the first thing in the document is the ground the rest sits on.
    let vb = src
        .split_once("viewBox=\"")
        .and_then(|(_, r)| r.split_once('"'))
        .map(|(v, _)| v)
        .ok_or_else(|| format!("no viewBox in {src_rel}"))?;
    let nums: Vec<f64> = vb
        .trim()
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .map(|s| s.parse().unwrap_or(f64::NAN))
        .collect();
    let f1 = |i: usize| fixed(nums.get(i).copied().unwrap_or(f64::NAN), 1);
    let bg = format!(
        "  <rect id=\"display-bg\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{PANEL}\"/>",
        f1(0),
        f1(1),
        f1(2),
        f1(3)
    );

    let mut out = insert_after_tag(src, "<?xml", "?>", &note);
    out = insert_after_tag(&out, "<svg", ">", &bg);
    if let Some(i) = out.find("<title>") {
        if let Some(j) = out[i..].find("</title>") {
            let inner = &out[i + 7..i + j];
            if !inner.contains('<') {
                out = format!("{}<title>{inner} — display only</title>{}", &out[..i], &out[i + j + 8..]);
            }
        }
    }
    out = out.replace("stroke=\"#000000\"", &format!("stroke=\"{SLIT}\""));
    let mut res = String::with_capacity(out.len());
    let mut last = 0;
    for (a, b) in stroke_widths(&out) {
        res.push_str(&out[last..a]);
        res.push_str(STROKE);
        last = b;
    }
    res.push_str(&out[last..]);
    Ok(res)
}

/// The source path as the comment states it: repo-relative, so the note names a
/// file a reader can actually open rather than wherever this was run from
/// (Node's path.relative(cwd, path.resolve(src))).
fn relative(src: &str) -> String {
    let cwd = std::env::current_dir().expect("no working directory");
    let mut abs = PathBuf::new();
    for c in cwd.join(src).components() {
        match c {
            Component::ParentDir => {
                abs.pop();
            }
            Component::CurDir => {}
            c => abs.push(c),
        }
    }
    let (a, b): (Vec<_>, Vec<_>) = (cwd.components().collect(), abs.components().collect());
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let mut rel = PathBuf::new();
    for _ in common..a.len() {
        rel.push("..");
    }
    for c in &b[common..] {
        rel.push(c);
    }
    rel.to_string_lossy().into_owned()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(src) = args.first() else {
        eprintln!("usage: preview CUTFILE.svg [OUT.svg]");
        eprintln!("  Writes previews/<name> unless an output path is given.");
        std::process::exit(2);
    };
    let name = Path::new(src).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let out = args.get(1).cloned().unwrap_or_else(|| format!("previews/{name}"));
    let text = std::fs::read_to_string(src).unwrap_or_else(|e| {
        eprintln!("cannot read {src}: {e}");
        std::process::exit(1)
    });
    let svg = build(&text, &relative(src)).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1)
    });

    // Geometry is not this tool's to change. Checked rather than trusted, the way
    // make-preview checks the same thing next door.
    if path_data(&text) != path_data(&svg) {
        eprintln!("  path data changed — refusing to write");
        std::process::exit(1);
    }
    if let Some(dir) = Path::new(&out).parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).unwrap_or_else(|e| {
            eprintln!("cannot create {}: {e}", dir.display());
            std::process::exit(1)
        });
    }
    std::fs::write(&out, &svg).unwrap_or_else(|e| {
        eprintln!("cannot write {out}: {e}");
        std::process::exit(1)
    });
    // kB as JavaScript counted string length: UTF-16 code units
    println!("  {out}  ({} kB)", fixed(svg.encode_utf16().count() as f64 / 1024.0, 1));
}
