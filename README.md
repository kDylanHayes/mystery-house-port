# Mystery House port

A port of *Hi-Res Adventure #1: Mystery House* (Ken and Roberta Williams, On-Line Systems, 1980;
public domain since 1987), built from the
[Mystery House decompilation](https://github.com/kDylanHayes/mystery-house-decompilation)
rather than from the Apple II code.

**Status:** the engine, in Rust, plays every conformance script exactly as the original
(transcripts and pixel hashes), with the documented bug fixes as an option, and runs in a
window on macOS, Windows and Linux.

## Playing

The [releases](https://github.com/kDylanHayes/mystery-house-port/releases) have the game ready
to run:

| System | Package | |
|---|---|---|
| macOS 11 or later (Apple silicon and Intel) | `mystery-house-<version>-macos-universal.zip` | `Mystery House.app`; not signed, so the first time, Control-click it and choose Open |
| Windows (x86-64) | `mystery-house-<version>-windows-x86_64.zip` | `mystery-house.exe`; not signed, so SmartScreen may ask first (More info, Run anyway) |
| Linux (x86-64, X11 or Wayland) | `mystery-house-<version>-linux-x86_64.tar.gz` | `./mystery-house` |

Or build and run it from the source (with [Rust](https://rustup.rs)):

```bash
cargo run --release -p mystery-house-app
```

It starts as the disk does: HELLO's credit for The Public Domain Exchange, the Sierra title
loading, the 1987 public-domain notice (a key goes on), then the loader's menu (`G` for the
game, `I` for the instructions); the 1980 title shows for a moment as the game loads. Any key
skips ahead to the notice, Esc to the loader. Type commands as on the
Apple: two words, verb first (`GO STAIRS`, `LOOK SINK`); RETURN on an empty line flips to the
text page and back.

| Key | |
|---|---|
| Left, Right, Home, End, Backspace, Delete | edit the line (lower case is typed as capitals) |
| Up, Down | earlier commands |
| Esc | clear the line |
| F1 | the instructions, at any time (RETURN for the next page, Esc to close) |
| F2 | white on black, or NTSC colour |
| F3 | square pixels in whole multiples, or a 4:3 picture |
| F11, Alt+Return | full screen |
| Cmd+1-9 (Ctrl on Windows and Linux) | save slot |

`SAVE GAME` and `RESTORE GAME` use the current slot. Each slot is a file in the original's
format (`SAVED GAME 1` ... `SAVED GAME 9`, the 2255 bytes the Apple BSAVEs), so saves move to
and from an Apple II or an emulator; an empty slot holds the disk's saved game, as the original
disk does. Saves and settings live in the platform's data folder (`~/Library/Application
Support/Mystery House` on macOS); `mystery-house --help` lists the command-line options
(`--boot`, `--loader`, `--skip-loader`, `--bugfix`, `--colour`, `--4:3`, `--fullscreen`,
`--slot N`, `--dir PATH`).

## Building and checking

```bash
cargo test
```

```bash
cargo run --release -p mh-play
```

The first plays every script in `decomp/play/` as a test; the second does the same and prints
the first difference of each script that fails. `mh-play house deaths` checks only those, and
`mh-play --play <file> [--loader] [--bugfix] [--pixels] [--save-in F] [--save-out F]` plays
any command file and prints the transcript (and the pixel hash at every prompt), optionally
starting from a save file and writing the save file at the end.

| Crate | |
|---|---|
| `crates/engine` (`mystery-house`) | the engine: loads `gamedata.json`, runs the loader and the turn loop, draws the screen; headless, fed one line at a time |
| `crates/play` (`mh-play`) | the script runner and the `cargo test` checks |
| `crates/app` (`mystery-house`) | the window (winit + pixels): the Apple's text page with its own 5 x 7 font, the line editor, white or NTSC colour, scaling, save slots and settings |

## What the port works from

`decomp/` is a pinned copy of what the decompilation provides for ports, from its `v1.1` tag
(`decomp/MANIFEST` records the tag, the commit and every file's SHA-256):

| File | |
|---|---|
| `decomp/PORTING.md` | the engine specification: screen, line drawing, pictures, shapes, text, the turn loop, parser, rules, saving, the start-up sequence, and how to check a port |
| `decomp/gamedata.json` | all the game's data: words, messages, rooms, objects, rules, pictures as strokes, shapes, the starting state, the disk's saved game, the loader's pages, the two title pictures |
| `decomp/play/*.txt` | conformance scripts: the solution, a tour of every room, every death and ending, unknown words, the loader and instructions, the bug fixes |
| `decomp/play/*.expected.txt` | what the original prints for each script |
| `decomp/play/*.pixels.txt` | a hash of what the player sees at every prompt |

The port is correct when every script gives the expected transcript and pixel hashes
(`decomp/PORTING.md`, "Checking a port"). The original's bugs are part of its behaviour; the
fixes the decompilation documents may be offered as an option, checked by `bugfix.txt`.

## Releasing

Pushing a tag `v*` runs `.github/workflows/release.yml`: on macOS, Windows and Linux it runs the
tests, builds the game (on macOS one app for both processors) and publishes the three packages
as a GitHub release. Running the workflow by hand builds the packages without publishing them.

```bash
git tag v1.0.1 && git push origin v1.0.1
```

## Updating from the decompilation

Fixes to the specification or the data go into the decompilation, which gets a new tag; then:

```bash
tools/sync-decomp.sh ../mystery-house-decomp v1.2
```

```bash
tools/sync-decomp.sh --verify
```

The first copies the files from that tag (exactly as the tag holds them) and rewrites
`decomp/MANIFEST`; the second checks the copies against it (CI runs it). Never edit `decomp/`
by hand.

## Licence

The game is public domain (1987 release). This project's own work is under the Unlicense
(`LICENSE`), as is the decompilation's.
