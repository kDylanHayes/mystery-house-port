# Porting Mystery House

What a port needs to behave exactly like the original, and how to check that it does. The
game is a small engine running a database. Port the engine described here, load
`assets/text/gamedata.json`, and the game follows. Every statement below is checked by the
tests, or is the source (`src/adventure.s`, routine names in brackets) read directly.

## What to use

| | |
|---|---|
| `assets/text/gamedata.json` | the whole database: words, messages, rooms, objects, rules, pictures as strokes, shapes as vectors, floor slots, the starting state, the disk's saved game, the loader's menu and instruction pages, the two title pictures (fields below) |
| `tools/pictures.py` | the reference renderer: the game's line drawing, pictures, shapes and room composition, pixel for pixel (`Canvas`, `draw_picture`, `draw_shape`, `room_canvas`) |
| `tests/play/*.txt` | conformance scripts: the solution, a tour of every room, every death and ending, unknown words, the loader and instructions, the bug fixes ("Checking a port") |
| `tests/play/*.expected.txt` | what the game prints for each script |
| `tests/play/*.pixels.txt` | what the player sees at every prompt, as a pixel hash a port can compute (below) |
| NOTES.md, `src/adventure.s` | the details and the source of truth |

The original's bugs are part of its behaviour. NOTES.md ("Quirks", "Bug-fix build") lists them.
A faithful port keeps them, and may offer the fixes `./build.sh --bugfix` makes as an option;
`tests/play/bugfix.txt` checks either way.

## Before the game

The disk boots through three programs before the game. They are all on the disk and all
public domain; a port can show them or start straight at the game.

1. **`HELLO`** (Applesoft, `src/basic/HELLO.bas`): a scrolling credit for The Public Domain
   Exchange, then `RUN MYSTERY.HELLO`.
2. **`MYSTERY.HELLO`** (`src/basic/MYSTERY.HELLO.bas`): shows the Sierra title picture, which
   it loads with `AUTO LOAD OBJ` (`titles.1987`, `assets/screens/title_1987.png`). After a pause it prints the
   1987 public-domain notice, waits for a key and starts the loader.
3. **`AUTO LOAD OBJ`** (`src/autoload.s`): the title picture with four text lines below it.
   It prints `loader.menu` (19 empty lines, then `ENTER G FOR GAME OR I FOR INSTRUCTIONS`) and
   reads a line:
   - `G...` starts the game;
   - `I...` shows the instructions: six full-screen text pages (`loader.pages`), each after
     the screen is cleared, each ending with `PRESS RETURN TO CONTINUE` (the last `PRESS RETURN
     TO BEGIN PLAY`) and waiting for a line; then the game;
   - anything else prints the menu again.

   The game is started with a carriage return and `BRUN ADVENTURE` (so one empty line
   comes before the game's own five). As it loads over the hi-res page, the 1980 On-Line
   Systems title shows for a moment (`titles.1980`, `assets/screens/title_1980.png`), then
   `init_game` clears the screen.

**The title pictures** are in `gamedata.json` as `titles.1987` and `titles.1980`, each the
whole 280 x 192 hi-res page:
- `pixels`: 192 rows from the top, each 35 bytes in hex, eight pixels a byte with the leftmost
  in bit 7 (1 = lit).
- `colour_bits`: 192 rows, each 5 bytes in hex: for the row's 40 groups of 7 pixels, the hi-res
  byte's bit 7 (leftmost group in bit 7), which shifts the NTSC artefact colours of that group
  (`pictures.hires_color`). The 1987 title has none set; the 1980 title uses them. A
  black-and-white port can ignore them.

Drawn from `pixels`, the 1987 title gives `intro.pixels.txt`'s hashes at the loader's prompts.

## The screen

An Apple II in hi-res mixed mode:
- **Picture:** a 280 x 160 monochrome area, with y = 0 at the top. Everything the game draws is white on black.
- **Text:** four 40-column lines below the picture.

The game draws only in white, so a port can treat the picture as one bit per pixel. The real
machine shows NTSC artefact colours on single pixels, which `pictures.hires_color` imitates.
They are a property of the display, not of the game.

### Lines (`hglin`, Applesoft HGLIN)

The game's lines are not Bresenham lines. Each step moves one pixel across or one pixel up or
down, never diagonally, so a slanted line is a staircase:

```
line from the pen (x0, y0) to (x1, y1):
    dx = |x1 - x0|, dy = |y1 - y0|
    step_x = -1 if x1 < x0 else +1;  step_y = +1 if y1 > y0 else -1
    err = dx - dy
    plot (x, y) = the pen                      # the start point is plotted again
    repeat dx + dy times:
        if err >= 0: x += step_x; err -= dy
        else:        y += step_y; err += dx
        plot (x, y)
    the pen is now (x1, y1)
```

`pictures.Canvas.line` is this, and it matches the game on every picture.

### Pictures (`draw_picture`)

A picture in `gamedata.json` is a list of strokes, each a list of `[x, y]` points:
- **Strokes:** the first point of a stroke is plotted; each later point is joined to the one before by a line.
- **Offset:** an object's picture is drawn offset by `(dx, dy)`. The offset is added in 8 bits, so a coordinate past 255 wraps round.
- **Clamp:** x is then capped at 255 and y at 160.
- **Pen carry-over:** the game remembers whether the pen is up across pictures. If `continues` is true (only picture 92), the first stroke does not start with a plot: its first point is a line from wherever the last picture left the pen.

### Shapes (`draw_shape`, Applesoft DRAW)

A few small objects are shapes. Each is a list of `[plot, direction]` vectors, with direction
0 up, 1 right, 2 down and 3 left. To draw a shape:
1. Start at the object's `(x, y)`.
2. For each vector, plot the current pixel if `plot` is 1, then move one pixel in its direction.

A shape moves only the screen position of the pen. The point a following line is measured
from stays at the shape's `(x, y)`; `Canvas` keeps both positions.

### The screen edges

Nothing the game draws reaches an edge: pictures are clamped to 255 x 160 and every shape is
drawn well inside. The Apple's routines would wrap a pen that ran off a side (past column 279
to column 0, and the rows round too), where `Canvas` simply drops the pixel. Neither happens
in the game, so a port need not copy either.

### What is on screen (`draw_room`, `draw_objects`)

Before every command prompt, the screen is cleared. If the light is on (`LAMPSTAT` = 0),
`draw_room` then draws:
1. The room's current picture (`RCURPIC`).
2. Each object in the current room, in `objects` order:
   - an `OMOVED` object (one that was taken and dropped) is drawn in a floor slot when the
     room shows its permanent view (`RCURPIC` = `RBASEPIC`). A close-up skips these objects.
     Every redraw starts at the first of the 21 `floor_slots` and gives the next slot to each
     dropped object in the room, in that same order;
   - any other object is drawn at `(OXCOORD, OYCOORD)` when `RCURPIC` is in its `ORPICSTA`
     list;
   - in both cases it is drawn as picture `OPIC`, or as shape `OPIC` when `shape` is true.

In the dark nothing is drawn: the picture area stays black. `pictures.room_canvas` does all of
this. The tests rebuild the screen at every command prompt of every script from the game state
and compare it with the game's pixels.

## Text

- **Messages:** printed by number from `messages`. Lines break at spaces so the first line
  is at most 39 characters (`word_wrap`). Its margin then grows by the previous margin plus
  one, a latent bug: no message is long enough to show it.
- **Built-in messages:** 7, 37, 127 and 137 are built into the code (`MESSLSTR`). `messages`
  holds the lines the game prints. The `MESSAGES` file words 37, 127 and 137 differently, and
  those file lines are never shown (7 happens to match).
- **Room description:** printed after the picture every turn (`RMSG`).
- **Prompt:** `print_prompt` prints `--------------- ENTER COMMAND` with no carriage return.
  `get_line` then calls `GETLN`, which prints the monitor prompt in `$33`. The game sets
  that to `?`, so the line on screen is `--------------- ENTER COMMAND?` and the typed
  command follows the question mark. `GETLN1` (the play-again question, and reading a
  message) prints no prompt. The transcripts flush that line and record the typed command
  as `> COMMAND`. They do not repeat the keys the monitor echoes after the question mark.
- **Start of a game:** five empty lines, then the first room.
- **DOS commands:** the lines the game prints to DOS (`BLOAD ...`, `READ MESSAGES...`,
  starting with Ctrl-D) are not shown, and a port has none.
- **Upper case:** the game's words are upper case and the original keyboard typed only
  capitals; a lower-case word matches nothing. A port should upper-case what is typed (the
  scripts are upper case).

## A turn (`main`)

```
init_game: turn = 1, room = 1, the 20 switches = 0, light on        (gamedata "start")
repeat:
    draw the screen (above), print the room's description
    read a command (below)
    command rules: the first that matches runs its actions; none -> message 37
    turn rules: every one that matches runs its actions
    turn += 1
```

**Reading a command** (`read_command`):
- The line is split at spaces. The first 8 letters of a word are kept and padded with spaces
  to 8, and that whole key has to equal the dictionary entry. `NOR` and `NORTH` are different
  words.
- The first word must be a verb and the second a noun; NOUN VERB is not accepted. Words after
  the second are ignored. Verb 1 is `verbs[0]` and noun 1 is `nouns[0]`.
- A missing noun is noun 70, eight spaces.
- If the verb is unknown, the game prints `I DONT KNOW HOW TO <word> SOMETHING` and asks again.
  If the noun is unknown, it prints `I DONT KNOW HOW TO <verb> A <noun>` and asks again.
  Each word is padded with spaces to 8 letters, as typed, upper case. Neither counts as a turn.
  The game asks again from the same screen, without another room description.
- An empty line flips between the picture and a full text page, and the game reads another
  line at once: just the `?` prompt, no `ENTER COMMAND` line, no turn. A port may skip the
  page flip, but must ask again the same way (`intro.txt` presses RETURN twice in the game).

**Rules** (`run_command_rules`, `run_turn_rules`):
- A rule matches when its room, verb and noun each equal the current room, verb and noun, or
  are 254 (any). Its conditions must then all hold. Room 1 is `rooms[0]`. Switch 0 is the first
  of the 20 switches. An object id is that entry's `OBJECTNO`.
- Conditions:

| Op | Holds when |
|---|---|
| `OBJ_IN_ROOM x y` | object x's room is y (254 = carried, 253 = nowhere) |
| `TURNS_GT n` | the turn counter is below 256 and greater than n (from turn 256 it is never true: a bug) |
| `SWITCH_EQ x y` | switch x equals y |
| `VIEW_IS n` | the picture on screen (`shown_picture`) is n: `draw_room` sets it to the room's current picture at the start of each turn, so a view changed during the turn counts from the next one |
| `OBJ_PIC_IS x y` | object x's picture is y |

- Actions:

| Op | Does |
|---|---|
| `ADD_SWITCH x y`, `SUB_SWITCH x y` | switch y += x / -= x (8 bits) |
| `SET_SWITCH x y` | switch x = y |
| `INVENTORY` | print the `OMSG` message of each carried object, in table order (nothing if none) |
| `PUT_OBJ x y` | object x's room = y |
| `PLACE_OBJ x y a b` | object x's room = y, position = (a, b) |
| `GOTO_ROOM x` | the room left goes back to its permanent view; the room becomes x |
| `SET_VIEW x` | the current room's current picture = x (a close-up) |
| `SET_BASE_VIEW x` | the current room's permanent and current picture = x |
| `RESET_VIEW` | the current picture = the permanent one |
| `SET_ROOM_VIEW x y` | room x's permanent and current picture = y |
| `SET_OBJ_PIC x y` | object y's picture = x |
| `MESSAGE x` | print message x |
| `LAMP_ON`, `LAMP_OFF` | the light (draws or hides the pictures) |
| `GO_N` ... `GO_D` | leave by that exit. Exit 0 prints message 137 and the current picture stays as it is. Any other exit returns the room being left to its permanent view, then makes the exit's room the current one. Either way, the rest of the rule's actions are skipped |
| `GET` | take the object named by the noun, walking `objects` in order. An object counts when its noun matches and its room is the current room. `OFIXED` prints message 151 and ends the search. `OMOVED`, or an `ORPICSTA` list that contains the room's current picture, is carried, becomes `OMOVED`, and ends the search. Any other match is passed over. Nothing taken: message 152 |
| `DROP` | the first carried object with that noun goes to this room as `OMOVED`; none carried: message 37 |
| `SAVE_GAME`, `RESTORE_GAME` | save or load the state (below); RESTORE also takes back the turn counter, which the turn's `+1` then advances |
| `GAME_OVER` | print `WOULD YOU LIKE TO PLAY AGAIN` and read a line: `N...` prints message 140 and the game ends; anything else restores the starting objects and rooms, prints `PRESS RETURN TO BEGIN PLAY` (it does not wait), and starts a new game at `init_game` |
| `QUIT` | print message 140; the game ends |

`gamedata.json` gives each rule as `room`, `verb`, `noun`, `conditions` and `actions`, with
the operation names above. Each `args` list is those operands in the order written here:
`SUB_SWITCH` 1, 3 subtracts 1 from switch 3, and `SET_OBJ_PIC` 97, 10 sets object 10's picture
to 97.

**Darkness and the timers** are not in the engine; they are turn rules (`assets/text/rules.txt`,
`$4400`-`$4465`). While the light is off the picture area stays black. `GET` still uses the
room's current picture.

## Saving

`SAVE GAME` writes the 2255 bytes at `$0830-$10FE` to the file `SAVED GAME`, and `RESTORE GAME`
loads them back. A port that wants to read and write the original's save files needs these
fields:

| Address | Field |
|---|---|
| `$0852` | current room |
| `$0853-$0854` | turn counter (low, high) |
| `$0834`, `$0835`, `$0839` | the last verb and noun, the picture on screen (`shown_picture`) |
| `$083A-$084D` | the 20 switches |
| `$089D` | the light: 0 on, 1 off |
| `$0900-` | the object table, records as in NOTES.md ("Rooms and objects"), in `gamedata.json` order (each object's `addr`) |
| `$0D00 + 10 * n` | room n's record; `RBASEPIC` and `RCURPIC` are the ones that change |

`src/saved_game.txt` is the disk's saved game written out field by field (`tools/savefile.py`),
which shows the format at work.

The disk comes with a saved game, and `RESTORE GAME` before any `SAVE GAME` loads it.
`gamedata.json`'s `saved_game` is that game in a port's terms. A port starts with it in its
save slot: `room`, `turn`, `switches`, `light`, every object's `OROOM`, `OPIC`, `OXCOORD`,
`OYCOORD` and `OSTATUS`, and every room's `RBASEPIC` and `RCURPIC`. It also has `verb` and
`noun`: the save file restores the last command's words too, and the turn rules that run
after a `RESTORE GAME` see those (here `SAVE GAME`, verb 34 and noun 88), and `shown_picture`,
the picture on screen that `VIEW_IS` tests until the next turn draws the room. `deaths.txt` and
`bugfix.txt` start from it.

## Checking a port

Play each script in `tests/play/`, typing each command when the game asks for input:

| Script | Start at | Compare with |
|---|---|---|
| `walkthrough.txt`, `errors.txt`, `house.txt`, `deaths.txt` | the game (`init_game`) | `<script>.expected.txt`, `<script>.pixels.txt` |
| `intro.txt` | the loader's menu (step 3 of "Before the game") | `intro.expected.txt`, `intro.pixels.txt` (at the loader's prompts the screen is the 1987 title, `titles.1987`) |
| `bugfix.txt` | the game | a faithful port: `bugfix.original.expected.txt`, `bugfix.original.pixels.txt`; with the bug fixes: `bugfix.expected.txt`, `bugfix.pixels.txt` |

In a script, lines starting `#` are comments and a line `<CR>` means RETURN on an empty line.
Then compare two things.

**The transcript:**
- Everything printed, as lines split at carriage returns, with trailing spaces removed.
- At every request for input (commands, the play-again answer and the loader's questions
  alike), the game's pending text is flushed and a line `> ` + the command, upper case, is
  added (`> ` alone for `<CR>`).
- Clearing the text screen (the loader's `HOME`, before each instruction page) is a line
  `<HOME>`.
- It must equal `<script>.expected.txt`.

**The screen:**
- At every request for input, take the 280 x 160 picture area as 44800 bytes, one per pixel
  (1 lit, 0 dark), row by row from the top.
- Its SHA-256, first 16 hex digits, must equal the next line of `<script>.pixels.txt`.
- `play.pixel_hash` computes it from Apple memory; from a port's framebuffer it is:

```python
hashlib.sha256(bytes(pixels[y][x] for y in range(160) for x in range(280))).hexdigest()[:16]
```

At the play-again prompt the screen still shows the last turn's picture: the game redraws only
at the start of a turn. A script ends when its commands run out, or when the game ends (QUIT,
a rude word, or N to play again).

`*.screens.txt` hashes the Apple's raw screen memory, colour bits and all, and is only for the
emulator.
