//! Mystery House in a window: the engine, the Apple II's screen and keyboard, save slots.

// A release build on Windows opens no console window beside the game's.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod boot;
mod font;
mod render;
mod settings;
mod store;
mod text;

use std::sync::Arc;

use web_time::{Duration, Instant};

use mystery_house::{savefile, Display, Game, GameData, Options, Output, Prompt, Start};
use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Fullscreen, Window, WindowId};

use boot::BootPlayer;
use render::{Aspect, Colour, Rgb, View};
use settings::{Begin, Settings};
use store::{warn, Store};
use text::{LineEditor, TextPage};

/// How long the 1980 title stays up as the game starts (any key skips it).
const TITLE_TIME: Duration = Duration::from_secs(2);
/// The cursor flashes at this half-period.
const BLINK: Duration = Duration::from_millis(400);

/// Sent to the event loop: in a web page the GPU is set up asynchronously.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
enum UserEvent {
    Pixels(Pixels<'static>),
}

struct App {
    data: &'static GameData,
    game: Game<'static>,
    settings: Settings,
    store: Store,
    /// For the GPU set up asynchronously in a web page (None in the tests).
    proxy: Option<EventLoopProxy<UserEvent>>,
    /// The text page as the game has printed it.
    page: TextPage,
    editor: LineEditor,
    /// The instruction pages on demand (F1): which page.
    instructions: Option<usize>,
    /// HELLO and MYSTERY.HELLO while they play, and when their clock started.
    boot: Option<(BootPlayer, Instant)>,
    title_until: Option<Instant>,
    saves_seen: u32,
    screen: Vec<Rgb>,
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    modifiers: ModifiersState,
    blink_from: Instant,
}

impl App {
    fn new(
        data: &'static GameData,
        settings: Settings,
        store: Store,
        proxy: Option<EventLoopProxy<UserEvent>>,
    ) -> App {
        let start = match settings.begin {
            Begin::Boot | Begin::Loader => Start::Loader,
            Begin::Game => Start::Game,
        };
        let boot = (settings.begin == Begin::Boot).then(|| (BootPlayer::new(data), Instant::now()));
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
            store,
            proxy,
            page: TextPage::default(),
            editor: LineEditor::new(),
            instructions: None,
            boot,
            title_until: None,
            saves_seen: 0,
            screen: vec![[0; 3]; render::W * render::H],
            window: None,
            pixels: None,
            modifiers: ModifiersState::empty(),
            blink_from: Instant::now(),
        };
        app.load_slot();
        if app.boot.is_none() {
            app.pump();
        }
        app
    }

    /// A new game, as the program starts (in a web page, where there is no quitting).
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    fn restart(&mut self) {
        let fresh = App::new(
            self.data,
            self.settings.clone(),
            self.store.clone(),
            self.proxy.clone(),
        );
        self.game = fresh.game;
        self.page = fresh.page;
        self.editor = fresh.editor;
        self.boot = fresh.boot;
        self.saves_seen = fresh.saves_seen;
        self.title_until = None;
        self.instructions = None;
    }

    /// Play the boot timeline up to now; when it reaches the loader, the loader takes over
    /// the text page as the boot left it.
    fn tick_boot(&mut self) {
        let Some((player, since)) = &mut self.boot else {
            return;
        };
        let ms = since.elapsed().as_secs_f64() * 1000.0;
        if player.advance(ms, &mut self.page) == boot::State::Loader {
            self.boot = None;
            self.pump();
        }
    }

    /// A key while the boot plays: Esc skips it all; GET's key goes on to the loader; any
    /// other key skips ahead to the notice GET waits on.
    fn boot_key(&mut self, escape: bool) {
        let Some((player, since)) = &mut self.boot else {
            return;
        };
        if !matches!(player.state(), boot::State::Key(..)) {
            player.skip_to_key(&mut self.page);
            if !escape {
                return;
            }
        }
        player.key_pressed();
        *since = Instant::now();
        self.tick_boot();
    }

    fn slot_name(&self) -> String {
        format!("SAVED GAME {}", self.settings.slot)
    }

    /// Put the slot's save file in the game's save file; an empty slot holds the disk's.
    fn load_slot(&mut self) {
        let state = match self.store.read(&self.slot_name()) {
            Some(file) => savefile::read(self.data, &file).unwrap_or_else(|e| {
                warn(&format!(
                    "{}: {e}; the slot starts with the disk's saved game",
                    self.slot_name()
                ));
                self.game.disk_saved_game_state()
            }),
            None => self.game.disk_saved_game_state(),
        };
        self.game.set_saved_game(state);
        self.saves_seen = self.game.saves();
    }

    fn save_settings(&self) {
        if let Err(e) = self
            .store
            .write("settings.txt", self.settings.to_text().as_bytes())
        {
            warn(&e);
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
            if let Err(e) = self.store.write(&self.slot_name(), &file) {
                warn(&e);
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
            if w.fullscreen().is_some() != self.settings.fullscreen {
                w.set_fullscreen(
                    self.settings
                        .fullscreen
                        .then_some(Fullscreen::Borderless(None)),
                );
            }
        }
    }

    fn redraw(&mut self) {
        self.tick_boot();
        let now = Instant::now();
        let lit = ((now - self.blink_from).as_millis() / BLINK.as_millis()).is_multiple_of(2);
        let screen = self.game.screen();
        let (mut pixels, mut groups) = (screen.pixels(), screen.colour_bits());
        let (view, page, cursor) = if let Some((player, _)) = &self.boot {
            if player.text {
                let cursor = match player.state() {
                    boot::State::Key(r, c) if lit => Some((r, c)),
                    _ => None,
                };
                (View::Text, self.page.clone(), cursor)
            } else {
                (pixels, groups) = (&player.pixels[..], &player.colour_bits[..]);
                (View::Graphics, TextPage::default(), None)
            }
        } else if self.title_until.is_some() {
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
            warn(&format!("render: {e}"));
        }
    }

    fn key(&mut self, event: &KeyEvent, el: &ActiveEventLoop) {
        self.blink_from = Instant::now();
        if self.title_until.take().is_some() {
            return;
        }
        let m = self.modifiers;
        // Alt too: a browser keeps Cmd/Ctrl+1-9 for its tabs
        let command = m.super_key() || m.control_key() || m.alt_key();
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
        if self.boot.is_some() {
            self.boot_key(event.logical_key == Key::Named(NamedKey::Escape));
            return;
        }
        let Some(prompt) = self.game.waiting() else {
            // the game is over: a key quits, or in a web page starts again
            #[cfg(not(target_arch = "wasm32"))]
            el.exit();
            #[cfg(target_arch = "wasm32")]
            self.restart();
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
            // what the key types: winit's text, or in a web page (which gives none) the key
            Key::Character(c) => self.editor.insert(event.text.as_deref().unwrap_or(c)),
            Key::Named(NamedKey::Space) => self.editor.insert(" "),
            _ => {}
        }
    }

    fn toggle_fullscreen(&mut self) {
        self.settings.fullscreen = !self.settings.fullscreen;
        self.save_settings();
        self.update_window();
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes().with_title(self.title());
        // A window opens at 3x; in a web page the page's CSS sizes the canvas.
        #[cfg(not(target_arch = "wasm32"))]
        let attrs = attrs
            .with_inner_size(winit::dpi::LogicalSize::new(
                render::W as f64 * 3.0,
                render::H as f64 * 3.0,
            ))
            .with_min_inner_size(winit::dpi::LogicalSize::new(
                render::W as f64,
                render::H as f64,
            ));
        #[cfg(target_arch = "wasm32")]
        let attrs = {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;
            // the page's <canvas id="mystery-house">, or a new one at the end of the page
            let canvas = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("mystery-house"))
                .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok());
            attrs
                .with_append(canvas.is_none())
                .with_canvas(canvas)
                .with_prevent_default(true)
                .with_focusable(true)
        };
        let window = Arc::new(el.create_window(attrs).expect("cannot open a window"));
        let size = window.inner_size();
        let (w, h) = (size.width.max(1), size.height.max(1));
        let surface = SurfaceTexture::new(w, h, window.clone());
        self.window = Some(window.clone());
        self.update_window();
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.pixels = Some(Pixels::new(w, h, surface).expect("cannot start the GPU"));
        }
        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowExtWebSys;
            if let Some(c) = window.canvas() {
                let _ = c.focus();
            }
            let proxy = self.proxy.clone();
            wasm_bindgen_futures::spawn_local(async move {
                // Canvases take no sRGB format, so the colours go through unconverted.
                let format = pixels::wgpu::TextureFormat::Rgba8Unorm;
                let built = pixels::PixelsBuilder::new(w, h, surface)
                    .texture_format(format)
                    .surface_texture_format(format)
                    .build_async()
                    .await;
                match built {
                    Ok(p) => {
                        if let Some(proxy) = proxy {
                            let _ = proxy.send_event(UserEvent::Pixels(p));
                        }
                    }
                    Err(e) => warn(&format!("cannot start the GPU: {e}")),
                }
            });
        }
    }

    fn user_event(&mut self, _: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Pixels(mut p) => {
                // the canvas may have been laid out meanwhile
                if let Some(w) = &self.window {
                    let size = w.inner_size();
                    if size.width > 0 && size.height > 0 {
                        let _ = p.resize_surface(size.width, size.height);
                        let _ = p.resize_buffer(size.width, size.height);
                    }
                    w.request_redraw();
                }
                self.pixels = Some(p);
            }
        }
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
                            warn(&format!("cannot resize to {}x{}", size.width, size.height));
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
        if self
            .boot
            .as_ref()
            .is_some_and(|(p, _)| p.state() == boot::State::Running)
        {
            next = next.min(now + Duration::from_millis(16));
        }
        el.set_control_flow(ControlFlow::WaitUntil(next));
    }
}

fn main() {
    let json = include_str!("../../../decomp/gamedata.json");
    let data: &'static GameData = Box::leak(Box::new(
        GameData::from_json(json).expect("decomp/gamedata.json"),
    ));
    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .expect("cannot start the event loop");
    let proxy = event_loop.create_proxy();

    #[cfg(not(target_arch = "wasm32"))]
    {
        let args: Vec<String> = std::env::args().skip(1).collect();
        // The options once to find --dir, then over the settings kept there.
        let dir = match Settings::default().apply_args(&args) {
            Ok(dir) => dir.unwrap_or_else(Store::default_dir),
            Err(msg) => {
                eprintln!("{msg}");
                let help = args.iter().any(|a| a == "-h" || a == "--help");
                std::process::exit(if help { 0 } else { 2 });
            }
        };
        let store = Store::new(dir);
        let settings = settings_from(&store, &args);
        let mut app = App::new(data, settings, store, Some(proxy));
        if let Err(e) = event_loop.run_app(&mut app) {
            eprintln!("{e}");
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        console_error_panic_hook::set_once();
        // The page's query gives the options: ?skip-loader&colour is --skip-loader --colour.
        let query = web_sys::window()
            .and_then(|w| w.location().search().ok())
            .unwrap_or_default();
        let args: Vec<String> = query
            .trim_start_matches('?')
            .split('&')
            .filter(|a| !a.is_empty())
            .map(|a| format!("--{}", a.replace('=', " ")))
            .flat_map(|a| a.split(' ').map(String::from).collect::<Vec<_>>())
            .collect();
        let store = Store;
        let mut settings = settings_from(&store, &args);
        settings.fullscreen = false; // a page may go full screen only on a key or click
        event_loop.spawn_app(App::new(data, settings, store, Some(proxy)));
    }
}

/// The settings kept in the store, with the options on top.
fn settings_from(store: &Store, args: &[String]) -> Settings {
    let text = store.read("settings.txt").unwrap_or_default();
    let mut settings = Settings::from_text(&String::from_utf8_lossy(&text));
    if let Err(msg) = settings.apply_args(args) {
        warn(&msg);
    }
    settings
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Screens as the window would show them, written as raw RGB to $MH_SNAPSHOTS
    /// (`cargo test -p mystery-house-app snapshots -- --ignored`).
    #[test]
    #[ignore]
    fn snapshots() {
        let out =
            std::path::PathBuf::from(std::env::var("MH_SNAPSHOTS").expect("set MH_SNAPSHOTS"));
        let json = include_str!("../../../decomp/gamedata.json");
        let data: &'static GameData = Box::leak(Box::new(GameData::from_json(json).unwrap()));
        let dir = std::env::temp_dir().join(format!("mh-snap-{}", std::process::id()));
        let store = || Store::new(dir.clone());
        let shot = |app: &mut App, name: &str, colour: Colour| {
            app.settings.colour = colour;
            app.blink_from = Instant::now();
            app.redraw();
            let bytes: Vec<u8> = app.screen.iter().flatten().copied().collect();
            std::fs::write(out.join(format!("{name}.rgb")), bytes).unwrap();
        };
        // The boot, at moments of its timeline.
        let mut app = App::new(data, Settings::default(), store(), None);
        for (ms, name) in [
            (1500, "hello_slide"),
            (4000, "hello_address"),
            (6500, "hello_credits"),
            (13500, "title_loading"),
        ] {
            app.boot.as_mut().unwrap().1 = Instant::now() - Duration::from_millis(ms);
            shot(&mut app, name, Colour::White);
        }
        app.boot_key(false);
        shot(&mut app, "notice", Colour::White);
        app.boot_key(false);
        assert!(app.boot.is_none());
        shot(&mut app, "loader_after_boot", Colour::White);

        let settings = Settings {
            begin: Begin::Loader,
            ..Settings::default()
        };
        let mut app = App::new(data, settings, store(), None);
        shot(&mut app, "loader", Colour::White);
        shot(&mut app, "loader_ntsc", Colour::Ntsc);
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
