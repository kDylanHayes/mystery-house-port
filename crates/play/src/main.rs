//! `mh-play`: check the conformance scripts, or play one and print what it produced.
//!
//! ```text
//! mh-play                         check every script (exit status 1 if any fails)
//! mh-play house deaths            check those scripts
//! mh-play --play <file> [--loader] [--bugfix] [--pixels] [--save-in <f>] [--save-out <f>]
//!                                 play a command file and print the transcript
//!                                 (and the pixel hashes), starting with save file <f>
//!                                 instead of the disk's, and writing the save file at
//!                                 the end (both in the original's SAVED GAME format)
//! ```

use std::process::ExitCode;

use mh_play::{check, decomp_dir, load_data, play_from, read_commands, SCRIPTS};
use mystery_house::{Options, Start};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let decomp = decomp_dir();
    let data = load_data(&decomp);

    if let Some(i) = args.iter().position(|a| a == "--play") {
        let Some(path) = args.get(i + 1) else {
            eprintln!("--play needs a command file");
            return ExitCode::from(2);
        };
        let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
        let start = if args.iter().any(|a| a == "--loader") {
            Start::Loader
        } else {
            Start::Game
        };
        let options = Options {
            bugfix: args.iter().any(|a| a == "--bugfix"),
        };
        let arg = |name: &str| {
            args.iter()
                .position(|a| a == name)
                .and_then(|i| args.get(i + 1))
        };
        let save_in =
            arg("--save-in").map(|f| std::fs::read(f).unwrap_or_else(|e| panic!("{f}: {e}")));
        let rec = play_from(
            &data,
            start,
            options,
            save_in.as_deref(),
            &read_commands(&text),
        );
        if let Some(f) = arg("--save-out") {
            std::fs::write(f, &rec.saved_game).unwrap_or_else(|e| panic!("{f}: {e}"));
        }
        for line in &rec.transcript {
            println!("{line}");
        }
        if args.iter().any(|a| a == "--pixels") {
            for hash in &rec.pixel_hashes {
                println!("{hash}");
            }
        }
        eprintln!("-- stopped: {}", rec.stopped);
        return ExitCode::SUCCESS;
    }

    let mut failed = 0;
    for script in SCRIPTS
        .iter()
        .filter(|s| args.is_empty() || args.iter().any(|a| a == s.name))
    {
        let label = format!("{} ({})", script.name, script.expected);
        let result = check(&data, &decomp, script);
        if result.passed() {
            println!("ok   {label}");
        } else {
            failed += 1;
            println!("FAIL {label}: stopped at {}", result.recording.stopped);
            for e in &result.errors {
                print!("{e}");
            }
        }
    }
    if failed > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
