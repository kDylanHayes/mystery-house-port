//! The player's settings: kept in `settings.txt` next to the save files, changed from the
//! command line or the keyboard.

use std::fs;
use std::path::{Path, PathBuf};

use crate::render::{Aspect, Colour};

pub const USAGE: &str = "usage: mystery-house [options]

  --loader / --skip-loader   start at the 1987 loader's menu, or straight at the game
  --bugfix / --original      the documented bug fixes on, or the original's bugs
  --colour / --white         NTSC colour, or white on black
  --square / --4:3           square pixels in whole multiples, or a 4:3 picture
  --fullscreen               start in full screen
  --slot N                   use save slot N (1-9)
  --dir PATH                 keep saves and settings in PATH

Keys: F1 instructions, F2 colour, F3 4:3, F11 or Alt+Return full screen,
Cmd/Ctrl+1-9 save slot, Up/Down earlier commands, Esc clears the line.
Saves are the original's SAVED GAME files, one per slot.";

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub loader: bool,
    pub bugfix: bool,
    pub colour: Colour,
    pub aspect: Aspect,
    pub fullscreen: bool,
    pub slot: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            loader: true,
            bugfix: false,
            colour: Colour::White,
            aspect: Aspect::Square,
            fullscreen: false,
            slot: 1,
        }
    }
}

/// Where saves and settings live: the platform's data directory.
pub fn default_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Mystery House")
}

impl Settings {
    pub fn load(dir: &Path) -> Settings {
        let mut s = Settings::default();
        if let Ok(text) = fs::read_to_string(dir.join("settings.txt")) {
            for line in text.lines() {
                if let Some((k, v)) = line.split_once('=') {
                    s.set(k.trim(), v.trim());
                }
            }
        }
        s
    }

    fn set(&mut self, key: &str, value: &str) {
        let on = value == "on";
        match key {
            "loader" => self.loader = on,
            "bugfix" => self.bugfix = on,
            "colour" => {
                self.colour = if value == "ntsc" {
                    Colour::Ntsc
                } else {
                    Colour::White
                }
            }
            "aspect" => {
                self.aspect = if value == "4:3" {
                    Aspect::FourThree
                } else {
                    Aspect::Square
                }
            }
            "fullscreen" => self.fullscreen = on,
            "slot" => {
                self.slot = value
                    .parse()
                    .ok()
                    .filter(|n| (1..=9).contains(n))
                    .unwrap_or(1)
            }
            _ => {}
        }
    }

    pub fn save(&self, dir: &Path) -> std::io::Result<()> {
        let onoff = |b: bool| if b { "on" } else { "off" };
        let text = format!(
            "loader = {}\nbugfix = {}\ncolour = {}\naspect = {}\nfullscreen = {}\nslot = {}\n",
            onoff(self.loader),
            onoff(self.bugfix),
            if self.colour == Colour::Ntsc {
                "ntsc"
            } else {
                "white"
            },
            if self.aspect == Aspect::FourThree {
                "4:3"
            } else {
                "square"
            },
            onoff(self.fullscreen),
            self.slot
        );
        fs::create_dir_all(dir)?;
        fs::write(dir.join("settings.txt"), text)
    }

    /// Apply command-line options; `--dir` is returned. Err is a message for the player.
    pub fn apply_args(&mut self, args: &[String]) -> Result<Option<PathBuf>, String> {
        let mut dir = None;
        let mut args = args.iter();
        while let Some(a) = args.next() {
            match a.as_str() {
                "--loader" => self.loader = true,
                "--skip-loader" => self.loader = false,
                "--bugfix" => self.bugfix = true,
                "--original" => self.bugfix = false,
                "--colour" | "--color" => self.colour = Colour::Ntsc,
                "--white" => self.colour = Colour::White,
                "--square" => self.aspect = Aspect::Square,
                "--4:3" => self.aspect = Aspect::FourThree,
                "--fullscreen" => self.fullscreen = true,
                "--slot" => {
                    let n = args
                        .next()
                        .and_then(|n| n.parse().ok())
                        .filter(|n| (1..=9).contains(n));
                    self.slot = n.ok_or("--slot needs a number from 1 to 9")?;
                }
                "--dir" => dir = Some(PathBuf::from(args.next().ok_or("--dir needs a path")?)),
                "-h" | "--help" => return Err(USAGE.to_string()),
                other => return Err(format!("unknown option {other}\n\n{USAGE}")),
            }
        }
        Ok(dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_survive_a_round_trip() {
        let dir = std::env::temp_dir().join(format!("mh-settings-{}", std::process::id()));
        let mut s = Settings::default();
        let args: Vec<String> = ["--skip-loader", "--colour", "--4:3", "--slot", "3"]
            .map(String::from)
            .into();
        assert_eq!(s.apply_args(&args), Ok(None));
        s.save(&dir).unwrap();
        assert_eq!(Settings::load(&dir), s);
        fs::remove_dir_all(dir).unwrap();
        assert!(s.apply_args(&["--slot".into(), "12".into()]).is_err());
    }
}
