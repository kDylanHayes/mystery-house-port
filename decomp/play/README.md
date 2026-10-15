# Play scripts

Command scripts played on the built game by `tests/test_play.py`, through the 6502 emulator in
`tools/play.py`. One command per line; `#` starts a comment, and a line `<CR>` is RETURN on an
empty line. In the transcripts a cleared text screen (the loader's HOME) is a line `<HOME>`.

| Script | What it plays |
|---|---|
| `walkthrough.txt` | the complete solution, ending with the winning message |
| `errors.txt` | words the game does not know (the replies are patched into text in the code) |
| `house.txt` | a tour of every room trying its rules: doors and gates in each state, the refusals, water, the trunk, the hidden wall, the tunnel and the forest |
| `deaths.txt` | every death and ending, darkness and the match, the first visit to the hall, SAVE GAME and RESTORE GAME (including the saved game on the disk) |
| `intro.txt` | from the 1987 loader (`play.py --boot`): its menu, the six instruction pages, then into the game, with RETURN on an empty line and words cut to 8 letters |
| `bugfix.txt` | played on the bug-fix build (`bugfix.*`) and on the original (`bugfix.original.*`) by `tests/test_bugfix.py`; not part of the coverage |

For each script, `<name>.expected.txt` is the transcript, `<name>.screens.txt` a short hash of
the hi-res screen memory at every prompt, and `<name>.pixels.txt` a hash of the picture the
player sees at every prompt, which a port can compute too (PORTING.md, "Checking a port"). `coverage.expected.txt` is what the scripts exercise
together (`tools/gamecov.py`): the counts, and the routines and rules that never run. Everything
listed there cannot run in the shipped game (NOTES.md, "Test coverage"), so a change that stops
some code or rule running, or a new script that reaches more, changes the report.

Things to know when writing a script:

* Many rules test the picture on screen. Looking at something (`LOOK SINK`, `READ NOTE`, ...)
  shows its picture instead of the room's, so the door rules stop matching until `LOOK ROOM`
  or a move. After turn 36 without a light the room is not drawn either, so light the candle
  early.
* `GET` takes only what the current picture shows: the matches after `LOOK CABINET`, the
  pitcher after `LOOK REFRIGERATOR`, the key after `LOOK HOLE`.
* After a death `Y` starts a new game; `RESTORE GAME` before any `SAVE GAME` loads the disk's
  saved game (the cemetery, turn 121, most things carried).

When a change is meant to alter what the game does, regenerate the expected files and read the
difference:

```bash
for s in walkthrough errors house deaths; do python3 tools/play.py extracted tests/play/$s.txt --transcript tests/play/$s.expected.txt --screen-hashes tests/play/$s.screens.txt --pixel-hashes tests/play/$s.pixels.txt; done
```

```bash
python3 tools/play.py extracted tests/play/intro.txt --boot --transcript tests/play/intro.expected.txt --screen-hashes tests/play/intro.screens.txt --pixel-hashes tests/play/intro.pixels.txt
```

```bash
python3 tools/play.py extracted tests/play/bugfix.txt --transcript tests/play/bugfix.original.expected.txt --screen-hashes tests/play/bugfix.original.screens.txt --pixel-hashes tests/play/bugfix.original.pixels.txt
```

```bash
python3 tools/play.py build/bugfix/files tests/play/bugfix.txt --transcript tests/play/bugfix.expected.txt --screen-hashes tests/play/bugfix.screens.txt --pixel-hashes tests/play/bugfix.pixels.txt
```

```bash
python3 tools/gamecov.py build extracted tests/play/walkthrough.txt tests/play/errors.txt tests/play/house.txt tests/play/deaths.txt tests/play/intro.txt --rules assets/text/rules.txt > tests/play/coverage.expected.txt
```

Add a new script to `SCRIPTS` in `tests/helpers.py`, a test for it in `tests/test_play.py`,
and to the coverage command above.
