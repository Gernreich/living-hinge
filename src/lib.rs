//! The living-hinge generator and what it needs to write numbers as Node did.
//!
//! The tools were JavaScript until 2026-11-02. Every shipped SVG was drawn by
//! Node, and .repro checks that each one redraws byte-identically. `jsfmt` is
//! JavaScript's number-to-text rules, which the files need: Rust formats
//! numbers differently. V8's own trigonometry (knotwork-soundholes carries a
//! port of it) is not needed here: std gives the same files; see generator.rs.

pub mod generator;
pub mod jsfmt;
