# Mystery House port

A port of Mystery House (Apple II, 1980; public domain since 1987) built from the decompilation
at ../mystery-house-decomp (GitHub kDylanHayes/mystery-house-decompilation). README.md has the
overview.

- Rust, as a Cargo workspace: `crates/engine` (the headless engine, crate `mystery-house`),
  `crates/play` (`mh-play`, the script runner) and `crates/app` (the window: winit 0.30 +
  pixels 0.17). `cargo test` plays every script. The engine
  stays headless and faithful; front-end features go in a front-end crate or behind `Options`,
  off by default, so the scripts stay the definition of done.
- v1 features agreed with the user: the boot sequence shown or skipped (the titles are in
  `gamedata.json` since decomp v1.1), instructions on demand, integer scaling/fullscreen/4:3,
  white-on-black and NTSC colour display, line editing with upper-casing and history, the
  bug-fix option, save slots, the original's `SAVED GAME` format, `--script` play. The boot (HELLO and
  MYSTERY.HELLO) replays `gamedata.json`'s `boot` timeline (decomp v1.3, `crates/app/src/boot.rs`).
- The app also builds for web pages (wasm32 + wasm-bindgen, `tools/build-web.sh`, `web/index.html`):
  `store.rs` keeps saves in localStorage there, and the GPU starts asynchronously (a user event).
  Check `cargo clippy -p mystery-house-app --target wasm32-unknown-unknown` too.
- The app's font is drawn for the port (`crates/app/src/font.rs`); the Apple's character ROM
  is not public domain.
- `decomp/` is a pinned copy from a decomp tag (`decomp/MANIFEST`). Never edit it. A fix to the
  specification or the data goes into the decompilation (its PORTING.md, gamedata.json or
  tests/play), gets a new tag there, and comes here with `tools/sync-decomp.sh <decomp> <tag>`.
- The port is checked by playing every script in `decomp/play/` and comparing the transcript
  and pixel hashes (`decomp/PORTING.md`, "Checking a port"). Keep that the definition of done.
- The original's bugs are behaviour: keep them by default; the documented fixes may be an
  option, checked by `bugfix.txt` against `bugfix.expected.txt`.
- Public-domain material only, as in the decompilation.
