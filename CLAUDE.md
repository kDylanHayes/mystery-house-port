# Mystery House port

A port of Mystery House (Apple II, 1980; public domain since 1987) built from the decompilation
at ../mystery-house-decomp (GitHub kDylanHayes/mystery-house-decompilation). README.md has the
overview.

- The language and platform are not decided yet: ask before writing engine code.
- `decomp/` is a pinned copy from a decomp tag (`decomp/MANIFEST`). Never edit it. A fix to the
  specification or the data goes into the decompilation (its PORTING.md, gamedata.json or
  tests/play), gets a new tag there, and comes here with `tools/sync-decomp.sh <decomp> <tag>`.
- The port is checked by playing every script in `decomp/play/` and comparing the transcript
  and pixel hashes (`decomp/PORTING.md`, "Checking a port"). Keep that the definition of done.
- The original's bugs are behaviour: keep them by default; the documented fixes may be an
  option, checked by `bugfix.txt` against `bugfix.expected.txt`.
- Public-domain material only, as in the decompilation.
