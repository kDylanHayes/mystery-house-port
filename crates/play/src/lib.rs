//! Plays the conformance scripts in `decomp/play/` on the engine and compares the transcript
//! and the pixel hashes with the expected files (`decomp/PORTING.md`, "Checking a port").

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use mystery_house::{savefile, Game, GameData, Options, Output, Prompt, Screen, Start};
use sha2::{Digest, Sha256};

/// The scripts, where each starts and which expected files it is compared with.
pub struct Script {
    pub name: &'static str,
    pub start: Start,
    pub bugfix: bool,
    /// The expected files' stem: `<expected>.expected.txt`, `<expected>.pixels.txt`.
    pub expected: &'static str,
}

pub const SCRIPTS: &[Script] = &[
    Script {
        name: "walkthrough",
        start: Start::Game,
        bugfix: false,
        expected: "walkthrough",
    },
    Script {
        name: "errors",
        start: Start::Game,
        bugfix: false,
        expected: "errors",
    },
    Script {
        name: "house",
        start: Start::Game,
        bugfix: false,
        expected: "house",
    },
    Script {
        name: "deaths",
        start: Start::Game,
        bugfix: false,
        expected: "deaths",
    },
    Script {
        name: "intro",
        start: Start::Loader,
        bugfix: false,
        expected: "intro",
    },
    Script {
        name: "bugfix",
        start: Start::Game,
        bugfix: false,
        expected: "bugfix.original",
    },
    Script {
        name: "bugfix",
        start: Start::Game,
        bugfix: true,
        expected: "bugfix",
    },
];

/// `decomp/` in this checkout.
pub fn decomp_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../decomp")
}

pub fn load_data(decomp: &Path) -> GameData {
    let path = decomp.join("gamedata.json");
    let json = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    GameData::from_json(&json).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// A command script: one command per line, `#` starts a comment, blank lines are skipped and a
/// line `<CR>` is RETURN on an empty line.
pub fn read_commands(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| l.split('#').next().unwrap().trim())
        .filter(|l| !l.is_empty())
        .map(|l| {
            if l == "<CR>" {
                String::new()
            } else {
                l.to_string()
            }
        })
        .collect()
}

/// SHA-256, first 16 hex digits, of the 280 x 160 picture area as one byte per pixel.
pub fn pixel_hash(screen: &Screen) -> String {
    let digest = Sha256::digest(screen.picture_area());
    digest[..8].iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// The transcript as PORTING.md defines it: printed lines with trailing spaces removed, a line
/// `> COMMAND` at each request for input, `<HOME>` for a cleared text screen.
#[derive(Default)]
struct Transcript {
    lines: Vec<String>,
    line: String,
}

impl Transcript {
    fn output(&mut self, out: Output) {
        match out {
            Output::Text(text) => {
                for c in text.chars() {
                    if c == '\n' {
                        self.lines.push(self.line.trim_end().to_string());
                        self.line.clear();
                    } else {
                        self.line.push(c);
                    }
                }
            }
            Output::Home => {
                self.flush();
                self.lines.push("<HOME>".into());
            }
        }
    }

    /// The pending text, unless it is only spaces (the reference player does the same).
    fn flush(&mut self) {
        if !self.line.trim().is_empty() {
            self.lines.push(self.line.trim_end().to_string());
        }
        self.line.clear();
    }
}

/// What a script produced: the transcript, and the pixel hash and kind of every prompt.
pub struct Recording {
    pub transcript: Vec<String>,
    pub prompts: Vec<(Prompt, String)>,
    /// Why the play stopped.
    pub stopped: String,
    /// The save file at the end, in the original's format.
    pub saved_game: Vec<u8>,
}

pub fn play(data: &GameData, start: Start, options: Options, commands: &[String]) -> Recording {
    play_from(data, start, options, None, commands)
}

/// Play with `saved_game` (the original's format) in the save file instead of the disk's.
pub fn play_from(
    data: &GameData,
    start: Start,
    options: Options,
    saved_game: Option<&[u8]>,
    commands: &[String],
) -> Recording {
    let mut game = Game::new(data, options, start);
    if let Some(file) = saved_game {
        let state = savefile::read(data, file).unwrap_or_else(|e| panic!("saved game: {e}"));
        game.set_saved_game(state);
    }
    let mut t = Transcript::default();
    let mut prompts = Vec::new();
    let mut commands = commands.iter();
    let stopped = loop {
        for out in game.take_output() {
            t.output(out);
        }
        let Some(prompt) = game.waiting() else {
            break "the game ended".to_string();
        };
        t.flush();
        prompts.push((prompt, pixel_hash(game.screen())));
        let Some(cmd) = commands.next() else {
            break "end of commands".to_string();
        };
        let cmd = cmd.to_uppercase();
        t.lines.push(format!("> {cmd}"));
        game.submit(&cmd);
    };
    t.flush();
    let saved_game = savefile::write(data, game.saved_game());
    Recording {
        transcript: t.lines,
        prompts,
        stopped,
        saved_game,
    }
}

/// The result of checking one script.
pub struct Check {
    pub recording: Recording,
    pub errors: Vec<String>,
}

impl Check {
    pub fn passed(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Play a script and compare it with its expected files.
pub fn check(data: &GameData, decomp: &Path, script: &Script) -> Check {
    let play_dir = decomp.join("play");
    let read = |name: String| {
        let path = play_dir.join(&name);
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    };
    let commands = read_commands(&read(format!("{}.txt", script.name)));
    let options = Options {
        bugfix: script.bugfix,
    };
    let recording = play(data, script.start, options, &commands);

    let mut errors = Vec::new();
    let expected = read(format!("{}.expected.txt", script.expected));
    let expected: Vec<&str> = expected.lines().collect();
    if let Some(e) = compare_transcript(&expected, &recording.transcript) {
        errors.push(e);
    }
    let hashes = read(format!("{}.pixels.txt", script.expected));
    let hashes: Vec<&str> = hashes.lines().collect();
    if let Some(e) = compare_pixels(&hashes, &recording) {
        errors.push(e);
    }
    Check { recording, errors }
}

/// The first difference, with a few lines of context, or None if the transcripts are equal.
fn compare_transcript(expected: &[&str], got: &[String]) -> Option<String> {
    let n = expected.iter().zip(got).position(|(e, g)| e != g);
    let n = match n {
        Some(n) => n,
        None if expected.len() == got.len() => return None,
        None => expected.len().min(got.len()),
    };
    let from = n.saturating_sub(4);
    let mut msg = format!(
        "transcript differs at line {} ({} lines expected, {} played)\n",
        n + 1,
        expected.len(),
        got.len()
    );
    for (i, line) in expected.iter().enumerate().skip(from).take(n - from + 3) {
        let _ = writeln!(msg, "  expected {:5}: {line}", i + 1);
    }
    for (i, line) in got.iter().enumerate().skip(from).take(n - from + 3) {
        let _ = writeln!(msg, "  played   {:5}: {line}", i + 1);
    }
    Some(msg)
}

fn compare_pixels(expected: &[&str], recording: &Recording) -> Option<String> {
    let mut bad = Vec::new();
    for (i, (_, hash)) in recording.prompts.iter().enumerate() {
        match expected.get(i) {
            Some(e) if e == hash => {}
            e => bad.push((i, e.copied().unwrap_or("(none)"), hash)),
        }
    }
    let count = expected.len() != recording.prompts.len();
    if bad.is_empty() && !count {
        return None;
    }
    let mut msg = format!(
        "pixel hashes: {} of {} prompts differ ({} expected)\n",
        bad.len(),
        recording.prompts.len(),
        expected.len()
    );
    for (i, e, g) in bad.iter().take(5) {
        // The transcript line of prompt i: the (i+1)th "> " line.
        let cmd = recording
            .transcript
            .iter()
            .filter(|l| l.starts_with("> "))
            .nth(*i)
            .map_or("(end)", |s| s.as_str());
        let _ = writeln!(
            msg,
            "  prompt {:4} (answered {cmd}): expected {e}, played {g}",
            i + 1
        );
    }
    Some(msg)
}
