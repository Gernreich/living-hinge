// =====================================================================
// panel convention -> annotated SVG figure for the guide
// =====================================================================
//
//   target/release/panel-convention [out.svg]
//
// Replaces the ASCII diagram that used to open section 1. The slits are not
// drawn by hand: this calls the generator (src/generator.rs) for a small
// straight panel and re-draws its actual output, so the figure cannot claim a
// geometry the tool does not produce. In particular the slits really do run out
// to x = 0 and x = width, which is the rule the section exists to state.
//
// The demo panel is deliberately short and coarse -- 60 x 84 with a 7mm pitch
// -- so the rows are legible at figure size. The proportions are illustrative;
// the shipped examples are 50 x 200.
//
// Rust since 2026-11-02 (it was panel-convention.js); the figure is unchanged.

use living_hinge::generator::{generate, Params};
use living_hinge::jsfmt::num;

const W: f64 = 60.0; // demo panel: width, length, margin
const L: f64 = 84.0;
const M: f64 = 7.0;

// ---------------------------------------------------------------- palette
const GOLD: &str = "#c9a227";
const TAB: &str = "#ab8a1e";
const CREAM: &str = "#faf7f0";
const INK: &str = "#1a1c1e";
const MUT: &str = "#5b6067";
const RULE: &str = "#8a8f96";

const S: f64 = 3.1; // label size in panel units

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;")
}

#[derive(Default)]
struct T<'a> {
    size: Option<f64>,
    fill: Option<&'a str>,
    anchor: Option<&'a str>,
    mono: bool,
}

fn txt(x: f64, y: f64, s: &str, o: T) -> String {
    format!(
        "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{}\"{}{}>{}</text>",
        num(x),
        num(y),
        num(o.size.unwrap_or(S)),
        o.fill.unwrap_or(INK),
        o.anchor.map(|a| format!(" text-anchor=\"{a}\"")).unwrap_or_default(),
        if o.mono { " font-family=\"ui-monospace,SFMono-Regular,Menlo,monospace\"" } else { "" },
        esc(s)
    )
}

/// The slits as the cut file writes them: `d="Mx1,y H x2"` inside the hinge-slits group.
fn parse_slits(svg: &str) -> Vec<[f64; 3]> {
    let start = svg.find("<g id=\"hinge-slits\"").expect("no hinge-slits group");
    let rest = &svg[start..];
    let group = &rest[..rest.find("</g>").expect("unclosed hinge-slits group")];
    let mut out = Vec::new();
    for piece in group.split("d=\"M").skip(1) {
        let body = &piece[..piece.find('"').unwrap()];
        let Some((xy, x2)) = body.split_once('H') else { continue };
        let Some((x1, y)) = xy.trim().split_once(',') else { continue };
        let p = |s: &str| s.trim().parse::<f64>().ok();
        if let (Some(a), Some(b), Some(c)) = (p(x1), p(y), p(x2)) {
            out.push([a, b, c]);
        }
    }
    out
}

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "panel-convention.svg".into());

    let opts = Params::from(&[
        ("width", W),
        ("height", L),
        ("margin", M),
        ("pitch", 7.0),
        ("bridge", 4.0),
        ("targetSlit", 14.0),
    ]);
    let r = generate("straight", "mm", &opts).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1)
    });
    let slits = parse_slits(&r.svg);
    if slits.is_empty() {
        eprintln!("no slits parsed");
        std::process::exit(1);
    }

    // ------------------------------------------------------------ frame
    // Frame sized to what the labels actually need: "y = length" reaches ~25 units
    // left of the panel, "(rows of cutouts)" ~45 to its right. Generous padding just
    // makes the figure mostly empty once the page scales it to the column width.
    let (pad_l, pad_r, pad_t, pad_b) = (30.0, 50.0, 26.0, 36.0);
    let (x0, y0, w, h) = (-pad_l, -pad_t, W + pad_l + pad_r, L + pad_t + pad_b);
    let mono = |fill: Option<&'static str>| T { mono: true, fill, ..Default::default() };
    let mid = |fill: Option<&'static str>| T { anchor: Some("middle"), mono: true, fill, ..Default::default() };

    let mut parts: Vec<String> = Vec::new();

    // panel: solid tabs top and bottom, hinge field between
    parts.push(format!("<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" fill=\"{GOLD}\"/>", num(W), num(L)));
    parts.push(format!("<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" fill=\"{TAB}\"/>", num(W), num(M)));
    parts.push(format!(
        "<rect x=\"0\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{TAB}\"/>",
        num(L - M),
        num(W),
        num(M)
    ));

    // the generator's own slits, drawn as the material they remove
    let lines: String = slits
        .iter()
        .map(|s| {
            format!("<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/>", num(s[0]), num(s[1]), num(s[2]), num(s[1]))
        })
        .collect();
    parts.push(format!("<g stroke=\"{CREAM}\" stroke-width=\"1.5\" stroke-linecap=\"butt\">{lines}</g>"));

    parts.push(format!(
        "<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"{INK}\" stroke-width=\"0.5\"/>",
        num(W),
        num(L)
    ));
    for y in [M, L - M] {
        parts.push(format!(
            "<line x1=\"0\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{INK}\" stroke-width=\"0.4\" stroke-dasharray=\"2 1.6\"/>",
            num(y),
            num(W),
            num(y)
        ));
    }

    // x axis, above the panel
    parts.push(txt(0.0, -14.0, "x = 0", mid(None)));
    parts.push(txt(W, -14.0, "x = width", mid(None)));
    for x in [0.0, W] {
        parts.push(format!(
            "<line x1=\"{}\" y1=\"-11\" x2=\"{}\" y2=\"-2\" stroke=\"{RULE}\" stroke-width=\"0.4\"/>",
            num(x),
            num(x)
        ));
    }

    // y axis, left of the panel
    let end = || T { anchor: Some("end"), mono: true, ..Default::default() };
    parts.push(txt(-6.0, 1.2, "y = 0", end()));
    parts.push(txt(-6.0, L + 1.2, "y = length", end()));

    // margin brackets and the field label, right of the panel
    let brace = |ya: f64, yb: f64, label: &str| {
        let bx = W + 7.0;
        format!(
            "<path d=\"M{} {}h3v{}h-3\" fill=\"none\" stroke=\"{RULE}\" stroke-width=\"0.5\"/>",
            num(bx),
            num(ya),
            num(yb - ya)
        ) + &txt(bx + 6.0, (ya + yb) / 2.0 + 1.1, label, mono(Some(MUT)))
    };
    parts.push(brace(0.0, M, "margin"));
    parts.push(brace(L - M, L, "margin"));
    parts.push(brace(M, L - M, "hinge field"));
    parts.push(txt(W + 13.0, L / 2.0 + 1.1 + S * 1.35, "(rows of cutouts)", mono(Some(MUT))));

    // width arrow, below
    let ay = L + 15.0;
    parts.push(format!(
        "<line x1=\"0\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{RULE}\" stroke-width=\"0.5\"/>",
        num(ay),
        num(W),
        num(ay)
    ));
    parts.push(format!(
        "<path d=\"M0 {}l3.4 -1.9v3.8z M{} {}l-3.4 -1.9v3.8z\" fill=\"{RULE}\"/>",
        num(ay),
        num(W),
        num(ay)
    ));
    parts.push(format!(
        "<rect x=\"{}\" y=\"{}\" width=\"16\" height=\"4.8\" fill=\"{CREAM}\"/>",
        num(W / 2.0 - 8.0),
        num(ay - 2.4)
    ));
    parts.push(txt(W / 2.0, ay + 1.15, "width", mid(Some(MUT))));
    parts.push(txt(
        W / 2.0,
        L + 27.0,
        "slits run ACROSS the width; the panel rolls ALONG the length",
        T { anchor: Some("middle"), fill: Some(INK), size: Some(S * 0.94), mono: false },
    ));

    let alt = "The panel convention: x runs across the width, y along the length. A solid end \
tab of the margin height sits at each end of the length, and between them the hinge field \
carries staggered rows of cutouts that run right out to both long edges.";

    let svg = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="{x0} {y0} {w} {h}"
     width="{w8}" height="{h8}" role="img" aria-label="{alt}">
  <title>Panel convention — width, length, margin, and the hinge field</title>
  <desc>Drawn by panel-convention.rs. The slits are real output from
src/generator.rs for a {W} x {L}mm straight panel with a {M}mm margin, so the
figure matches what the tool makes. Proportions are illustrative; the shipped examples
are 50 x 200mm.</desc>
  <rect x="{x0}" y="{y0}" width="{w}" height="{h}" fill="{CREAM}"/>
  <g font-family="ui-sans-serif,system-ui,-apple-system,Helvetica,Arial,sans-serif">
    {parts}
  </g>
</svg>
"#,
        x0 = num(x0),
        y0 = num(y0),
        w = num(w),
        h = num(h),
        w8 = num(w * 8.0),
        h8 = num(h * 8.0),
        alt = esc(alt),
        W = num(W),
        L = num(L),
        M = num(M),
        parts = parts.join("\n    "),
    );
    if let Err(e) = std::fs::write(&out, svg) {
        eprintln!("cannot write {out}: {e}");
        std::process::exit(1);
    }
    let stat = |k: &str| r.stats.iter().find(|(s, _)| s == k).map(|(_, v)| v.to_string()).unwrap_or_default();
    println!("panel {} x {}, margin {}, {} slits from the generator", num(W), num(L), num(M), slits.len());
    println!("rows {}  pitch {}  bridge {}  slit {}", stat("rows"), stat("pitch"), stat("bridge"), stat("slit"));
    println!("wrote {out}");
}
