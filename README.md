# Mystery House port

A port of *Hi-Res Adventure #1: Mystery House* (Ken and Roberta Williams, On-Line Systems, 1980;
public domain since 1987), built from the
[Mystery House decompilation](https://github.com/kDylanHayes/mystery-house-decompilation)
rather than from the Apple II code.

**Status:** just split off from the decompilation. The language and platform are not decided
yet, so there is no engine code. What is here is everything the port will be built and checked
against.

## What the port works from

`decomp/` is a pinned copy of what the decompilation provides for ports, from its `v1.0` tag
(`decomp/MANIFEST` records the commit and every file's SHA-256):

| File | |
|---|---|
| `decomp/PORTING.md` | the engine specification: screen, line drawing, pictures, shapes, text, the turn loop, parser, rules, saving, the start-up sequence, and how to check a port |
| `decomp/gamedata.json` | all the game's data: words, messages, rooms, objects, rules, pictures as strokes, shapes, the starting state, the disk's saved game, the loader's pages |
| `decomp/play/*.txt` | conformance scripts: the solution, a tour of every room, every death and ending, unknown words, the loader and instructions, the bug fixes |
| `decomp/play/*.expected.txt` | what the original prints for each script |
| `decomp/play/*.pixels.txt` | a hash of what the player sees at every prompt |

The port is correct when every script gives the expected transcript and pixel hashes
(`decomp/PORTING.md`, "Checking a port"). The original's bugs are part of its behaviour; the
fixes the decompilation documents may be offered as an option, checked by `bugfix.txt`.

## Updating from the decompilation

Fixes to the specification or the data go into the decompilation, which gets a new tag; then:

```bash
tools/sync-decomp.sh ../mystery-house-decomp v1.1
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
