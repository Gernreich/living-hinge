Three levels, most to least reliable.

  1. Don't ask me at all. For a plain size or pattern change, the script is the whole answer:

  cd ~/LaserMadeMusic/GIT/living-hinge && ./living-hinge.js -p wave -w 80 -l 300

  No session, no tokens, no risk of me re-deriving anything. --list and --help remind you of the options.

  2. Name the script. Works regardless of whether memory fires:

  ▎ Use ~/LaserMadeMusic/GIT/living-hinge/living-hinge.js to make a chevron hinge 80 × 300mm with a 10mm margin.

  3. Just ask normally. In a session started in ~/Claude, this should be enough:

  ▎ Make me a living hinge, wave pattern, 80 × 300mm.

  The memory index loads at session start and now carries a line pointing at the generator, so "living hinge" should surface it and I'll run the script rather than rebuild.

  The catch worth knowing, and it sharpened on 2026-08-06: that memory is scoped to the ~/Claude project directory, and the generator no longer lives there — it moved to ~/LaserMadeMusic/GIT/living-hinge to become its own repository. A session started in ~/Claude still loads the memory line pointing here. A session started in this directory, or anywhere else, does not — so use phrasing 2 with the full path.

  How to tell it went wrong: if I start writing geometry code, computing slit segments, or asking what bridge width you want, memory didn't fire. Say "use ~/LaserMadeMusic/GIT/living-hinge/living-hinge.js" and I'll switch over. The tell is me producing a script instead of running one.

  One thing to hand me in a new session that I won't otherwise know: your test-cut results. The defaults are still unvalidated, and that's recorded in the memory. Once you've cut a coupon, telling me "the 0.9mm ligament snapped in 3mm ply, 1.5mm held" is worth more
  than any of the above — I'll update the defaults in the generator and correct the memory so the next session starts from real numbers instead of my estimates.

