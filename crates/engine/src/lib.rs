//! The Mystery House engine, ported from the decompilation's specification
//! (`decomp/PORTING.md`). It runs headless: a front end (or the test runner) feeds it lines
//! when it asks for one, takes what it printed and reads its screen.
//!
//! ```text
//! let mut game = Game::new(&data, Options::default(), Start::Game);
//! while let Some(prompt) = game.waiting() {
//!     for out in game.take_output() { ... }
//!     game.submit(&line);
//! }
//! ```

mod data;
mod screen;

pub use data::{Action, Condition, GameData, Rule, Title};
pub use screen::{Screen, HEIGHT, PICTURE_HEIGHT, WIDTH};

use data::{ANY, CARRIED, OFIXED, OMOVED};

/// Where a game starts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Start {
    /// The 1987 loader's menu (`AUTO LOAD OBJ`).
    Loader,
    /// The game itself (`init_game`).
    Game,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    /// The documented fixes of the original's bugs (the decompilation's bug-fix build).
    pub bugfix: bool,
}

/// What the game is waiting for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Prompt {
    /// A line for the loader: the menu or an instruction page.
    Loader,
    /// A command (`GETLN`, after `ENTER COMMAND` and the `?`).
    Command,
    /// A line with no prompt (`GETLN1`): the play-again answer.
    Line,
}

/// What the game printed.
#[derive(Clone, Debug, PartialEq)]
pub enum Output {
    /// Text; `\n` is a carriage return.
    Text(String),
    /// The text screen was cleared (the loader's HOME).
    Home,
}

/// Everything `SAVE GAME` writes and `RESTORE GAME` loads back (the game's `$0830-$10FE`),
/// as far as it changes what the game does.
#[derive(Clone)]
pub struct State {
    pub room: u8,
    pub turn: u16,
    pub switches: [u8; 20],
    pub lamp_on: bool,
    pub verb: u8,
    pub noun: u8,
    /// The picture last put on screen (`shown_picture`): what `VIEW_IS` tests.
    pub shown_picture: u8,
    pub pen_up: bool,
    pub objects: Vec<ObjectState>,
    pub rooms: Vec<RoomState>,
}

#[derive(Clone, Copy)]
pub struct ObjectState {
    pub room: u8,
    pub picture: u8,
    pub x: u8,
    pub y: u8,
    pub status: u8,
}

#[derive(Clone, Copy)]
pub struct RoomState {
    pub base_picture: u8,
    pub current_picture: u8,
}

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Menu,
    Page(usize),
    Command,
    PlayAgain,
    Ended,
}

/// How a rule's actions ended.
#[derive(PartialEq)]
enum Flow {
    Next,
    /// A move or a restore: the rest of the rule's actions are skipped.
    Stop,
    /// `WOULD YOU LIKE TO PLAY AGAIN` is waiting for its answer.
    PlayAgain,
    /// Back to DOS.
    Quit,
}

pub struct Game<'a> {
    data: &'a GameData,
    options: Options,
    command_rules: Vec<Rule>,
    turn_rules: Vec<Rule>,
    state: State,
    /// The starting objects and rooms (the `STARTUP` file).
    startup: State,
    /// The save file.
    saved: State,
    screen: Screen,
    output: Vec<Output>,
    phase: Phase,
    /// RETURN on an empty line shows the full text page instead of the picture.
    text_page: bool,
}

impl<'a> Game<'a> {
    pub fn new(data: &'a GameData, options: Options, start: Start) -> Game<'a> {
        let startup = State {
            room: 1,
            turn: 1,
            switches: [0; 20],
            lamp_on: true,
            verb: 0,
            noun: 0,
            shown_picture: 0,
            pen_up: false,
            objects: data
                .objects
                .iter()
                .map(|o| ObjectState {
                    room: o.room,
                    picture: o.picture,
                    x: o.x,
                    y: o.y,
                    status: o.status,
                })
                .collect(),
            rooms: data
                .rooms
                .iter()
                .map(|r| RoomState {
                    base_picture: r.base_picture,
                    current_picture: r.current_picture,
                })
                .collect(),
        };
        let saved = Self::disk_saved_game(data, &startup);
        let (mut command_rules, mut turn_rules) =
            (data.command_rules.clone(), data.turn_rules.clone());
        if options.bugfix {
            fix_rules(&mut command_rules);
            fix_rules(&mut turn_rules);
        }
        let mut game = Game {
            data,
            options,
            command_rules,
            turn_rules,
            state: startup.clone(),
            startup,
            saved,
            screen: Screen::default(),
            output: Vec::new(),
            phase: Phase::Ended,
            text_page: false,
        };
        match start {
            Start::Loader => game.start_loader(),
            Start::Game => game.new_game(),
        }
        game
    }

    /// The disk's `SAVED GAME`, which `RESTORE GAME` loads before any `SAVE GAME`.
    fn disk_saved_game(data: &GameData, startup: &State) -> State {
        let s = &data.saved_game;
        let mut state = startup.clone();
        state.room = s.room;
        state.turn = s.turn;
        state.switches = s.switches;
        state.lamp_on = s.light;
        state.verb = s.verb;
        state.noun = s.noun;
        for &(id, room, picture, x, y, status) in &s.objects {
            if let Some(i) = data.objects.iter().position(|o| o.id == id) {
                state.objects[i] = ObjectState {
                    room,
                    picture,
                    x,
                    y,
                    status,
                };
            }
        }
        for &(n, base_picture, current_picture) in &s.rooms {
            state.rooms[n as usize - 1] = RoomState {
                base_picture,
                current_picture,
            };
        }
        state.shown_picture = s.shown_picture;
        state
    }

    /// What the game is waiting for, or None once it has ended.
    pub fn waiting(&self) -> Option<Prompt> {
        match self.phase {
            Phase::Menu | Phase::Page(_) => Some(Prompt::Loader),
            Phase::Command => Some(Prompt::Command),
            Phase::PlayAgain => Some(Prompt::Line),
            Phase::Ended => None,
        }
    }

    /// Everything printed since the last call.
    pub fn take_output(&mut self) -> Vec<Output> {
        std::mem::take(&mut self.output)
    }

    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    /// True while the full text page is shown instead of the picture.
    pub fn text_page(&self) -> bool {
        self.text_page
    }

    pub fn options(&self) -> Options {
        self.options
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    /// Answer the prompt with a typed line (upper-cased here, as the original keyboard did).
    pub fn submit(&mut self, line: &str) {
        let line = line.to_uppercase();
        match self.phase {
            Phase::Menu => match line.chars().next() {
                Some('G') => self.run_adventure(),
                Some('I') => self.show_page(0),
                _ => self.show_menu(),
            },
            Phase::Page(n) if n + 1 < self.data.loader.pages.len() => self.show_page(n + 1),
            Phase::Page(_) => self.run_adventure(),
            Phase::Command => self.command(&line),
            Phase::PlayAgain => {
                if line.starts_with('N') {
                    self.quit();
                } else {
                    // BLOAD STARTUP, HGR, then main.
                    let startup = self.startup.clone();
                    self.state = startup;
                    self.screen.clear();
                    self.print("PRESS RETURN TO BEGIN PLAY");
                    self.new_game();
                }
            }
            Phase::Ended => {}
        }
    }

    fn print(&mut self, text: &str) {
        if let Some(Output::Text(t)) = self.output.last_mut() {
            t.push_str(text);
        } else {
            self.output.push(Output::Text(text.to_string()));
        }
    }

    // ---- the loader -------------------------------------------------------------------------

    /// `MYSTERY.HELLO` has loaded `AUTO LOAD OBJ`, whose `$2000-$3FFF` is the 1987 title.
    fn start_loader(&mut self) {
        let t = &self.data.title_1987;
        self.screen.load_page(&t.pixels, &t.colour_bits);
        self.show_menu();
    }

    fn show_menu(&mut self) {
        let menu = self.data.loader.menu.join("\n");
        self.print(&menu);
        self.phase = Phase::Menu;
    }

    fn show_page(&mut self, n: usize) {
        self.output.push(Output::Home);
        let page = self.data.loader.pages[n].join("\n");
        self.print(&page);
        self.phase = Phase::Page(n);
    }

    /// `\r` and `BRUN ADVENTURE`, which loads the 1980 title over the screen; `init_game`
    /// and the first `draw_room` then clear it. A front end may hold it for a moment.
    fn run_adventure(&mut self) {
        self.print("\n");
        let t = &self.data.title_1980;
        self.screen.load_page(&t.pixels, &t.colour_bits);
        self.new_game();
    }

    // ---- the game ---------------------------------------------------------------------------

    /// `main`: `init_game`, then the first turn.
    fn new_game(&mut self) {
        let s = &mut self.state;
        s.turn = 1;
        s.room = 1;
        s.switches = [0; 20];
        s.lamp_on = true;
        self.print("\n\n\n\n\n");
        self.start_turn();
    }

    /// `draw_room`, the room's description and the prompt.
    fn start_turn(&mut self) {
        self.draw_room();
        let message = self.data.room(self.state.room).message;
        self.message(message);
        self.prompt();
    }

    /// `print_prompt` and GETLN's `?`.
    fn prompt(&mut self) {
        self.print("--------------- ENTER COMMAND?");
        self.phase = Phase::Command;
    }

    fn quit(&mut self) {
        self.message(140);
        self.phase = Phase::Ended;
    }

    fn command(&mut self, line: &str) {
        if line.is_empty() {
            self.text_page = !self.text_page;
            self.print("?");
            return;
        }
        let mut words = line.split(' ').filter(|w| !w.is_empty()).map(word_key);
        let verb_word = words.next().unwrap_or_else(|| word_key(""));
        let Some(verb) = lookup(&self.data.verbs, &verb_word) else {
            self.print(&format!("I DONT KNOW HOW TO {verb_word} SOMETHING\n"));
            self.prompt();
            return;
        };
        let noun_word = words.next().unwrap_or_else(|| word_key(""));
        let Some(noun) = lookup(&self.data.nouns, &noun_word) else {
            self.print(&format!("I DONT KNOW HOW TO {verb_word} A {noun_word}\n"));
            self.prompt();
            return;
        };
        self.state.verb = verb;
        self.state.noun = noun;

        match self.run_rules() {
            Flow::PlayAgain => {
                self.print("WOULD YOU LIKE TO PLAY AGAIN");
                self.phase = Phase::PlayAgain;
            }
            Flow::Quit => self.quit(),
            Flow::Next | Flow::Stop => {
                self.state.turn = self.state.turn.wrapping_add(1);
                self.start_turn();
            }
        }
    }

    /// The first command rule that matches, then every turn rule that matches. Play-again and
    /// quit leave at once (the original jumps away).
    fn run_rules(&mut self) -> Flow {
        let rules = std::mem::take(&mut self.command_rules);
        let mut flow = Flow::Next;
        let mut matched = false;
        for rule in &rules {
            if self.matches(rule) {
                matched = true;
                flow = self.run_actions(rule);
                break;
            }
        }
        self.command_rules = rules;
        if !matched {
            self.message(37);
        }
        if matches!(flow, Flow::PlayAgain | Flow::Quit) {
            return flow;
        }
        let rules = std::mem::take(&mut self.turn_rules);
        for rule in &rules {
            if self.matches(rule) {
                flow = self.run_actions(rule);
                if matches!(flow, Flow::PlayAgain | Flow::Quit) {
                    break;
                }
            }
        }
        self.turn_rules = rules;
        flow
    }

    fn matches(&self, rule: &Rule) -> bool {
        let s = &self.state;
        let header = |r: u8, v: u8| r == ANY || r == v;
        header(rule.room, s.room)
            && header(rule.verb, s.verb)
            && header(rule.noun, s.noun)
            && rule.conditions.iter().all(|c| self.holds(*c))
    }

    fn object_index(&self, id: u8) -> usize {
        self.data
            .objects
            .iter()
            .position(|o| o.id == id)
            .unwrap_or_else(|| panic!("object {id} does not exist (the original BRKs)"))
    }

    fn holds(&self, c: Condition) -> bool {
        let s = &self.state;
        match c {
            Condition::ObjInRoom(x, y) => s.objects[self.object_index(x)].room == y,
            Condition::TurnsGt(n) => {
                if s.turn >= 256 {
                    self.options.bugfix
                } else {
                    s.turn as u8 > n
                }
            }
            Condition::SwitchEq(x, y) => s.switches[x as usize] == y,
            Condition::ViewIs(n) => s.shown_picture == n,
            Condition::ObjPicIs(x, y) => s.objects[self.object_index(x)].picture == y,
        }
    }

    fn room_state(&mut self) -> &mut RoomState {
        &mut self.state.rooms[self.state.room as usize - 1]
    }

    fn run_actions(&mut self, rule: &Rule) -> Flow {
        for &action in &rule.actions {
            let flow = self.act(action);
            if flow != Flow::Next {
                return flow;
            }
        }
        Flow::Next
    }

    fn act(&mut self, action: Action) -> Flow {
        match action {
            Action::AddSwitch(x, y) => {
                let sw = &mut self.state.switches[y as usize];
                *sw = sw.wrapping_add(x);
            }
            Action::SubSwitch(x, y) => {
                let sw = &mut self.state.switches[y as usize];
                *sw = sw.wrapping_sub(x);
            }
            Action::SetSwitch(x, y) => self.state.switches[x as usize] = y,
            Action::Inventory => {
                for i in 0..self.data.objects.len() {
                    if self.state.objects[i].room == CARRIED {
                        self.message(self.data.objects[i].message);
                    }
                }
            }
            Action::PutObj(x, y) => {
                let i = self.object_index(x);
                self.state.objects[i].room = y;
            }
            Action::PlaceObj(x, y, a, b) => {
                let i = self.object_index(x);
                let o = &mut self.state.objects[i];
                (o.room, o.x, o.y) = (y, a, b);
            }
            Action::GotoRoom(x) => {
                self.reset_view();
                self.state.room = x;
            }
            Action::SetView(x) => self.room_state().current_picture = x,
            Action::SetBaseView(x) => {
                *self.room_state() = RoomState {
                    base_picture: x,
                    current_picture: x,
                }
            }
            Action::ResetView => self.reset_view(),
            Action::SetRoomView(x, y) => {
                self.state.rooms[x as usize - 1] = RoomState {
                    base_picture: y,
                    current_picture: y,
                };
            }
            Action::SetObjPic(x, y) => {
                let i = self.object_index(y);
                self.state.objects[i].picture = x;
            }
            Action::Message(x) => self.message(x),
            Action::LampOn => self.state.lamp_on = true,
            Action::LampOff => self.state.lamp_on = false,
            Action::Go(exit) => {
                let to = self.data.room(self.state.room).exits[exit];
                if to == 0 {
                    self.message(137);
                } else {
                    self.reset_view();
                    self.state.room = to;
                }
                return Flow::Stop;
            }
            Action::Get => self.get(),
            Action::Drop => self.drop_object(),
            Action::SaveGame => self.saved = self.state.clone(),
            Action::RestoreGame => {
                self.state = self.saved.clone();
                return Flow::Stop;
            }
            Action::GameOver => return Flow::PlayAgain,
            Action::Quit => return Flow::Quit,
        }
        Flow::Next
    }

    /// The current room goes back to its permanent view.
    fn reset_view(&mut self) {
        let r = self.room_state();
        r.current_picture = r.base_picture;
    }

    /// `AC3700`: take the object named by the noun.
    fn get(&mut self) {
        let current = self.state.rooms[self.state.room as usize - 1].current_picture;
        for (i, o) in self.data.objects.iter().enumerate() {
            let st = self.state.objects[i];
            if o.noun != self.state.noun || st.room != self.state.room {
                continue;
            }
            if st.status == OFIXED {
                self.message(151);
                return;
            }
            if st.status == OMOVED || o.in_pictures.contains(&current) {
                let st = &mut self.state.objects[i];
                st.room = CARRIED;
                st.status = OMOVED;
                return;
            }
        }
        self.message(152);
    }

    /// `AC3800`: drop the first carried object with the noun.
    fn drop_object(&mut self) {
        for (i, o) in self.data.objects.iter().enumerate() {
            let st = &mut self.state.objects[i];
            if o.noun == self.state.noun && st.room == CARRIED {
                st.room = self.state.room;
                st.status = OMOVED;
                return;
            }
        }
        self.message(37);
    }

    /// `MESSLSTR`: message n, word-wrapped.
    fn message(&mut self, n: u8) {
        let text = self.data.messages.get(&n).cloned().unwrap_or_default();
        self.print(&word_wrap(&text));
    }

    // ---- the screen -------------------------------------------------------------------------

    /// `draw_room`: clear the screen and, if the light is on, draw the room's current picture
    /// and its objects.
    fn draw_room(&mut self) {
        let room = self.state.rooms[self.state.room as usize - 1];
        self.state.shown_picture = room.current_picture;
        self.screen.clear();
        if !self.state.lamp_on {
            return;
        }
        self.draw_picture(room.current_picture, 0, 0);
        let mut slot = 0;
        for (i, o) in self.data.objects.iter().enumerate() {
            let st = self.state.objects[i];
            if st.room != self.state.room {
                continue;
            }
            if st.status == OMOVED {
                if room.base_picture == room.current_picture {
                    if let Some(&(x, y)) = self.data.floor_slots.get(slot) {
                        self.draw_object(o.shape, st.picture, x, y);
                    }
                    slot += 1;
                }
            } else if o.in_pictures.contains(&self.state.shown_picture) {
                self.draw_object(o.shape, st.picture, st.x, st.y);
            }
        }
    }

    fn draw_object(&mut self, shape: bool, picture: u8, x: u8, y: u8) {
        if shape {
            if let Some(vectors) = self.data.shapes.get(&picture) {
                self.screen.draw_shape(vectors, x, y);
            }
        } else {
            self.draw_picture(picture, x, y);
        }
    }

    fn draw_picture(&mut self, n: u8, dx: u8, dy: u8) {
        if let Some(p) = self.data.pictures.get(&n) {
            self.screen
                .draw_picture(&p.strokes, p.continues, dx, dy, &mut self.state.pen_up);
        }
    }
}

/// A word as the parser keeps it: the first 8 characters, padded with spaces.
fn word_key(word: &str) -> String {
    let mut key: String = word.chars().take(8).collect();
    while key.chars().count() < 8 {
        key.push(' ');
    }
    key
}

/// `lookup_word`: the number of the group holding the word (1 = the first group).
fn lookup(groups: &[Vec<String>], key: &str) -> Option<u8> {
    groups
        .iter()
        .position(|g| g.iter().any(|w| word_key(w) == key))
        .map(|i| i as u8 + 1)
}

/// `word_wrap`: break the line at spaces so the first line is at most 39 characters. After a
/// break the margin becomes the break's column plus the old margin plus one (a latent bug:
/// right for the second line only). Ends with a carriage return.
fn word_wrap(text: &str) -> String {
    let mut b: Vec<u8> = text.bytes().collect();
    b.push(b'\n');
    let mut y: u8 = 0;
    let mut margin: u8 = 39;
    loop {
        if b[y as usize] == b'\n' {
            break;
        }
        if y == margin {
            while b[y as usize] != b' ' {
                y = y.wrapping_sub(1);
            }
            b[y as usize] = b'\n';
            margin = y.wrapping_add(margin).wrapping_add(1);
        }
        y = y.wrapping_add(1);
    }
    b.truncate(y as usize + 1);
    String::from_utf8_lossy(&b).into_owned()
}

/// The bug-fix build's rules (the decompilation's `.ifdef BUGFIX` blocks).
fn fix_rules(rules: &mut [Rule]) {
    for rule in rules {
        match rule.addr {
            // WITH DAGGER at Joe without the dagger: only the message, Joe keeps his eyes.
            0x4ADD => rule.actions.retain(|a| matches!(a, Action::Message(_))),
            // OPEN TRUNK on the open trunk: IT IS OPEN.
            0x4F4B => {
                for a in &mut rule.actions {
                    if *a == Action::Message(82) {
                        *a = Action::Message(129);
                    }
                }
            }
            // The storage-room window ends the game, as the attic window does.
            0x4FEB => rule.actions.push(Action::GameOver),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_at_39() {
        let s = "YOU ARE IN THE FRONT YARD OF A LARGE ABANDONED VICTORIAN HOUSE.  STONE STEPS LEAD UP TO A WIDE PORCH";
        assert_eq!(
            word_wrap(s),
            "YOU ARE IN THE FRONT YARD OF A LARGE\nABANDONED VICTORIAN HOUSE.  STONE STEPS\nLEAD UP TO A WIDE PORCH\n"
        );
    }

    #[test]
    fn words_are_cut_to_8() {
        assert_eq!(word_key("FENCEPOSTS"), "FENCEPOS");
        assert_eq!(word_key("N"), "N       ");
    }
}
