//! Mystery House in a window: the engine, the Apple II's screen and keyboard, save slots.

mod font;
mod render;
mod settings;
mod text;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use mystery_house::{savefile, Display, Game, GameData, Options, Output, Prompt, Start};
use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Fullscreen, Window, WindowId};

use render::{Aspect, Colour, Rgb, View};
use settings::Settings;
use text::{LineEditor, TextPage};

/// How long the 1980 title stays up as the game starts (any key skips it).
const TITLE_TIME: Duration = Duration::from_secs(2);
/// The cursor flashes at this half-period.
const BLINK: Duration = Duration::from_millis(400);

struct App {
    data: &'static GameData,
    game: Game<'static>,
    settings: Settings,
    dir: PathBuf,
    /// The text page as the game has printed it.
    page: TextPage,
    editor: LineEditor,
    /// The instruction pages on demand (F1): which page.
    instructions: Option<usize>,
    title_until: Option<Instant>,
    saves_seen: u32,
    screen: Vec<Rgb>,
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    modifiers: ModifiersState,
    blink_from: Instant,
}

impl App {
    fn new(data: &'static GameData, settings: Settings, dir: PathBuf) -> App {
        let start = if settings.loader {
            Start::Loader
        } else {
            Start::Game
        };
        let game = Game::new(
            data,
            Options {
                bugfix: settings.bugfix,
            },
            start,
        );
        let mut app = App {
            data,
            game,
            settings,
            dir,
            page: TextPage::default(),
            editor: LineEditor::new(),
            instructions: None,
            title_until: None,
            saves_seen: 0,
            screen: vec![[0; 3]; render::W * render::H],
            window: None,
            pixels: None,
            modifiers: ModifiersState::empty(),
            blink_from: Instant::now(),
        };
        app.load_slot();
        app.pump();
        app
    }

    fn slot_path(&self) -> PathBuf {
        self.dir.join(format!("SAVED GAME {}", self.settings.slot))
    }

    /// Put the slot's save file in the game's save file; an empty slot holds the disk's.
    fn load_slot(&mut self) {
        let state = match std::fs::read(self.slot_path()) {
            Ok(file) => savefile::read(self.data, &file).unwrap_or_else(|e| {
                eprintln!(
                    "{}: {e}; the slot starts with the disk's saved game",
                    self.slot_path().display()
                );
                self.game.disk_saved_game_state()
            }),
            Err(_) => self.game.disk_saved_game_state(),
        };
        self.game.set_saved_game(state);
        self.saves_seen = self.game.saves();
    }

    fn save_settings(&self) {
        if let Err(e) = self.settings.save(&self.dir) {
            eprintln!("{}: {e}", self.dir.display());
        }
    }

    /// Take what the game printed, and write the save file if it saved.
    fn pump(&mut self) {
        for out in self.game.take_output() {
            match out {
                Output::Text(t) => self.page.print(&t),
                Output::Home => self.page.home(),
            }
        }
        if self.game.take_title_1980() {
            self.title_until = Some(Instant::now() + TITLE_TIME);
        }
        if self.game.saves() != self.saves_seen {
            self.saves_seen = self.game.saves();
            let file = savefile::write(self.data, self.game.saved_game());
            let written = std::fs::create_dir_all(&self.dir)
                .and_then(|_| std::fs::write(self.slot_path(), file));
            if let Err(e) = written {
                eprintln!("{}: {e}", self.slot_path().display());
            }
        }
        self.editor.start(&self.page);
    }

    fn title(&self) -> String {
        let mut t = format!("Mystery House - save slot {}", self.settings.slot);
        if self.settings.bugfix {
            t.push_str(" - bug fixes on");
        }
        t
    }

    fn update_window(&self) {
        if let Some(w) = &self.window {
            w.set_title(&self.title());
            w.set_fullscreen(
                self.settings
                    .fullscreen
                    .then_some(Fullscreen::Borderless(None)),
            );
        }
    }

    fn redraw(&mut self) {
        let now = Instant::now();
        let lit = ((now - self.blink_from).as_millis() / BLINK.as_millis()).is_multiple_of(2);
        let screen = self.game.screen();
        let (mut pixels, mut groups) = (screen.pixels(), screen.colour_bits());
        let (view, page, cursor) = if self.title_until.is_some() {
            let t = &self.data.title_1980;
            (pixels, groups) = (&t.pixels[..], &t.colour_bits[..]);
            (View::Graphics, TextPage::default(), None)
        } else if let Some(n) = self.instructions {
            let mut page = TextPage::default();
            page.home();
            page.print(&self.data.loader.pages[n].join("\n"));
            (View::Text, page, None)
        } else {
            let (page, at) = self.editor.view();
            let view = match self.game.display() {
                Display::Mixed => View::Mixed,
                Display::Text => View::Text,
            };
            let cursor = (self.game.waiting().is_some() && lit).then_some(at);
            (view, page, cursor)
        };
        render::compose(
            pixels,
            groups,
            &page,
            view,
            self.settings.colour,
            cursor,
            &mut self.screen,
        );
        let (Some(window), Some(p)) = (&self.window, &mut self.pixels) else {
            return;
        };
        let size = window.inner_size();
        render::blit(
            &self.screen,
            p.frame_mut(),
            size.width as usize,
            size.height as usize,
            self.settings.aspect,
        );
        if let Err(e) = p.render() {
            eprintln!("render: {e}");
        }
    }

    fn key(&mut self, event: &KeyEvent, el: &ActiveEventLoop) {
        self.blink_from = Instant::now();
        if self.title_until.take().is_some() {
            return;
        }
        let m = self.modifiers;
        let command = m.super_key() || m.control_key();
        match &event.logical_key {
            Key::Named(NamedKey::F1) => {
                self.instructions = if self.instructions.is_some() {
                    None
                } else {
                    Some(0)
                };
                return;
            }
            Key::Named(NamedKey::F2) => {
                self.settings.colour = match self.settings.colour {
                    Colour::White => Colour::Ntsc,
                    Colour::Ntsc => Colour::White,
                };
                self.save_settings();
                return;
            }
            Key::Named(NamedKey::F3) => {
                self.settings.aspect = match self.settings.aspect {
                    Aspect::Square => Aspect::FourThree,
                    Aspect::FourThree => Aspect::Square,
                };
                self.save_settings();
                return;
            }
            Key::Named(NamedKey::F11) => return self.toggle_fullscreen(),
            Key::Named(NamedKey::Enter) if m.alt_key() => return self.toggle_fullscreen(),
            Key::Character(c) if command => {
                match c.as_str() {
                    "q" if m.super_key() => el.exit(),
                    "f" if m.super_key() && m.control_key() => self.toggle_fullscreen(),
                    d => {
                        if let Some(n) = d.parse::<u8>().ok().filter(|n| (1..=9).contains(n)) {
                            self.settings.slot = n;
                            self.load_slot();
                            self.save_settings();
                            self.update_window();
                        }
                    }
                }
                return;
            }
            _ => {}
        }

        if let Some(n) = self.instructions {
            match &event.logical_key {
                Key::Named(NamedKey::Escape) => self.instructions = None,
                Key::Named(NamedKey::Enter | NamedKey::Space) => {
                    self.instructions = (n + 1 < self.data.loader.pages.len()).then_some(n + 1);
                }
                _ => {}
            }
            return;
        }
        let Some(prompt) = self.game.waiting() else {
            el.exit();
            return;
        };
        match &event.logical_key {
            Key::Named(NamedKey::Enter) => {
                let (line, page) = self.editor.enter(prompt == Prompt::Command);
                self.page = page;
                self.game.submit(&line);
                self.pump();
            }
            Key::Named(NamedKey::Backspace) => self.editor.backspace(),
            Key::Named(NamedKey::Delete) => self.editor.delete(),
            Key::Named(NamedKey::ArrowLeft) => self.editor.left(),
            Key::Named(NamedKey::ArrowRight) => self.editor.right(),
            Key::Named(NamedKey::Home) => self.editor.home(),
            Key::Named(NamedKey::End) => self.editor.end(),
            Key::Named(NamedKey::ArrowUp) => self.editor.browse(true),
            Key::Named(NamedKey::ArrowDown) => self.editor.browse(false),
            Key::Named(NamedKey::Escape) => self.editor.clear(),
            _ => {
                if let Some(t) = &event.text {
                    self.editor.insert(t);
                }
            }
        }
    }

    fn toggle_fullscreen(&mut self) {
        self.settings.fullscreen = !self.settings.fullscreen;
        self.save_settings();
        self.update_window();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title(self.title())
            .with_inner_size(LogicalSize::new(
                render::W as f64 * 3.0,
                render::H as f64 * 3.0,
            ))
            .with_min_inner_size(LogicalSize::new(render::W as f64, render::H as f64));
        let window = Arc::new(el.create_window(attrs).expect("cannot open a window"));
        let size = window.inner_size();
        let surface = SurfaceTexture::new(size.width, size.height, window.clone());
        let pixels = Pixels::new(size.width, size.height, surface).expect("cannot start the GPU");
        self.window = Some(window);
        self.pixels = Some(pixels);
        self.update_window();
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(size) => {
                if let Some(p) = &mut self.pixels {
                    if size.width > 0 && size.height > 0 {
                        let ok = p.resize_surface(size.width, size.height).is_ok()
                            && p.resize_buffer(size.width, size.height).is_ok();
                        if !ok {
                            eprintln!("cannot resize to {}x{}", size.width, size.height);
                        }
                    }
                }
            }
            WindowEvent::ModifiersChanged(m) => self.modifiers = m.state(),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                self.key(&event, el);
            }
            WindowEvent::RedrawRequested => {
                self.redraw();
                return;
            }
            _ => return,
        }
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn new_events(&mut self, _: &ActiveEventLoop, cause: StartCause) {
        if let StartCause::ResumeTimeReached { .. } = cause {
            if self.title_until.is_some_and(|t| Instant::now() >= t) {
                self.title_until = None;
            }
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        let now = Instant::now();
        let since = (now - self.blink_from).as_millis() % BLINK.as_millis();
        let mut next = now + BLINK - Duration::from_millis(since as u64);
        if let Some(t) = self.title_until {
            next = next.min(t);
        }
        el.set_control_flow(ControlFlow::WaitUntil(next));
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // The options once to find --dir, then over the settings kept there.
    let dir = match Settings::default().apply_args(&args) {
        Ok(dir) => dir.unwrap_or_else(settings::default_dir),
        Err(msg) => {
            eprintln!("{msg}");
            std::process::exit(if args.iter().any(|a| a == "-h" || a == "--help") {
                0
            } else {
                2
            });
        }
    };
    let mut settings = Settings::load(&dir);
    let _ = settings.apply_args(&args);
    let json = include_str!("../../../decomp/gamedata.json");
    let data: &'static GameData = Box::leak(Box::new(
        GameData::from_json(json).expect("decomp/gamedata.json"),
    ));
    let mut app = App::new(data, settings, dir);
    let event_loop = EventLoop::new().expect("cannot start the event loop");
    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("{e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Screens as the window would show them, written as raw RGB to $MH_SNAPSHOTS
    /// (`cargo test -p mystery-house-app snapshots -- --ignored`).
    #[test]
    #[ignore]
    fn snapshots() {
        let out = PathBuf::from(std::env::var("MH_SNAPSHOTS").expect("set MH_SNAPSHOTS"));
        let json = include_str!("../../../decomp/gamedata.json");
        let data: &'static GameData = Box::leak(Box::new(GameData::from_json(json).unwrap()));
        let dir = std::env::temp_dir().join(format!("mh-snap-{}", std::process::id()));
        let mut app = App::new(data, Settings::default(), dir);
        let shot = |app: &mut App, name: &str, colour: Colour| {
            app.settings.colour = colour;
            app.blink_from = Instant::now();
            app.redraw();
            let bytes: Vec<u8> = app.screen.iter().flatten().copied().collect();
            std::fs::write(out.join(format!("{name}.rgb")), bytes).unwrap();
        };
        shot(&mut app, "loader", Colour::White);
        app.editor.insert("G");
        let (l, p) = app.editor.enter(false);
        app.page = p;
        app.game.submit(&l);
        app.pump();
        shot(&mut app, "title_1980", Colour::Ntsc);
        app.title_until = None;
        for line in ["GO STAIRS", "OPEN DOOR", "N", "N"] {
            app.editor.insert(line);
            let (l, p) = app.editor.enter(true);
            app.page = p;
            app.game.submit(&l);
            app.pump();
        }
        app.editor.insert("look");
        shot(&mut app, "hall", Colour::White);
        shot(&mut app, "hall_ntsc", Colour::Ntsc);
        app.instructions = Some(1);
        shot(&mut app, "instructions", Colour::White);
    }
}
