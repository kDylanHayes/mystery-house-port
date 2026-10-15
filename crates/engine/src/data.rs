//! `gamedata.json`: the whole database (decomp/PORTING.md, "What to use").

use std::collections::HashMap;

use serde::Deserialize;

/// Object `OSTATUS` values (0 is movable).
pub const OMOVED: u8 = 1;
pub const OFIXED: u8 = 2;
/// `OROOM` of a carried object (253 is nowhere).
pub const CARRIED: u8 = 254;
/// A rule's room, verb or noun that matches anything.
pub const ANY: u8 = 254;

/// The game's database, as loaded from `gamedata.json`.
pub struct GameData {
    /// Word groups: verb n is `verbs[n - 1]`, its first word and then its synonyms.
    pub verbs: Vec<Vec<String>>,
    pub nouns: Vec<Vec<String>>,
    pub messages: HashMap<u8, String>,
    /// Room n is `rooms[n - 1]`.
    pub rooms: Vec<Room>,
    /// In table order, which is the order the game walks them in.
    pub objects: Vec<Object>,
    pub pictures: HashMap<u8, Picture>,
    /// Shape n: `[plot, direction]` vectors (direction 0 up, 1 right, 2 down, 3 left).
    pub shapes: HashMap<u8, Vec<(bool, u8)>>,
    pub command_rules: Vec<Rule>,
    pub turn_rules: Vec<Rule>,
    pub floor_slots: Vec<(u8, u8)>,
    pub saved_game: SavedGame,
    pub loader: Loader,
}

pub struct Room {
    pub number: u8,
    pub message: u8,
    /// N S E W U D; 0 = no exit.
    pub exits: [u8; 6],
    pub base_picture: u8,
    pub current_picture: u8,
}

pub struct Object {
    pub id: u8,
    pub noun: u8,
    pub room: u8,
    pub picture: u8,
    pub shape: bool,
    pub x: u8,
    pub y: u8,
    pub status: u8,
    /// Printed by INVENTORY.
    pub message: u8,
    /// The pictures it is drawn in (`ORPICSTA`).
    pub in_pictures: Vec<u8>,
}

pub struct Picture {
    /// Picture 92: the first stroke continues from wherever the pen is.
    pub continues: bool,
    pub strokes: Vec<Vec<(u8, u8)>>,
}

#[derive(Clone)]
pub struct Rule {
    /// The rule's address in the original, which names it (`rule_4FEB`).
    pub addr: u16,
    pub room: u8,
    pub verb: u8,
    pub noun: u8,
    pub conditions: Vec<Condition>,
    pub actions: Vec<Action>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Condition {
    ObjInRoom(u8, u8),
    TurnsGt(u8),
    SwitchEq(u8, u8),
    ViewIs(u8),
    ObjPicIs(u8, u8),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    /// switch y += x
    AddSwitch(u8, u8),
    /// switch y -= x
    SubSwitch(u8, u8),
    /// switch x = y
    SetSwitch(u8, u8),
    Inventory,
    PutObj(u8, u8),
    PlaceObj(u8, u8, u8, u8),
    GotoRoom(u8),
    SetView(u8),
    SetBaseView(u8),
    ResetView,
    /// room x's views = picture y
    SetRoomView(u8, u8),
    /// object y's picture = x
    SetObjPic(u8, u8),
    Message(u8),
    LampOn,
    LampOff,
    /// 0..6: N S E W U D
    Go(usize),
    Get,
    Drop,
    SaveGame,
    RestoreGame,
    GameOver,
    Quit,
}

/// The disk's saved game, in a port's terms (decomp/PORTING.md, "Saving").
pub struct SavedGame {
    pub room: u8,
    pub turn: u16,
    pub switches: [u8; 20],
    pub light: bool,
    pub verb: u8,
    pub noun: u8,
    /// (OBJECTNO, OROOM, OPIC, OXCOORD, OYCOORD, OSTATUS)
    pub objects: Vec<(u8, u8, u8, u8, u8, u8)>,
    /// (RROOMNO, RBASEPIC, RCURPIC)
    pub rooms: Vec<(u8, u8, u8)>,
}

/// The 1987 loader's text (`AUTO LOAD OBJ`).
pub struct Loader {
    pub menu: Vec<String>,
    pub pages: Vec<Vec<String>>,
}

impl GameData {
    pub fn from_json(json: &str) -> Result<GameData, String> {
        let raw: json::GameData = serde_json::from_str(json).map_err(|e| e.to_string())?;
        raw.convert()
    }

    pub fn room(&self, n: u8) -> &Room {
        &self.rooms[n as usize - 1]
    }
}

/// The file's own shape, converted into the types above.
mod json {
    #![allow(non_snake_case)]

    use super::*;

    #[derive(Deserialize)]
    pub struct GameData {
        verbs: Vec<Vec<String>>,
        nouns: Vec<Vec<String>>,
        messages: HashMap<String, String>,
        rooms: Vec<Room>,
        objects: Vec<Object>,
        pictures: HashMap<String, Picture>,
        shapes: HashMap<String, Vec<[u8; 2]>>,
        command_rules: Vec<Rule>,
        turn_rules: Vec<Rule>,
        floor_slots: Vec<[u8; 2]>,
        saved_game: SavedGame,
        loader: Loader,
    }

    #[derive(Deserialize)]
    struct Room {
        RROOMNO: u8,
        RMSG: u8,
        RN: u8,
        RS: u8,
        RE: u8,
        RW: u8,
        RU: u8,
        RD: u8,
        RBASEPIC: u8,
        RCURPIC: u8,
    }

    #[derive(Deserialize)]
    struct Object {
        OBJECTNO: u8,
        noun: u8,
        OROOM: u8,
        OPIC: u8,
        shape: bool,
        OXCOORD: u8,
        OYCOORD: u8,
        OSTATUS: u8,
        OMSG: u8,
        ORPICSTA: Vec<u8>,
    }

    #[derive(Deserialize)]
    struct Picture {
        continues: bool,
        strokes: Vec<Vec<[u8; 2]>>,
    }

    #[derive(Deserialize)]
    struct Rule {
        addr: u16,
        room: u8,
        verb: u8,
        noun: u8,
        conditions: Vec<Op>,
        actions: Vec<Op>,
    }

    #[derive(Deserialize)]
    struct Op {
        op: String,
        args: Vec<u8>,
    }

    #[derive(Deserialize)]
    struct SavedGame {
        room: u8,
        turn: u16,
        switches: [u8; 20],
        light: bool,
        verb: u8,
        noun: u8,
        objects: Vec<SavedObject>,
        rooms: Vec<SavedRoom>,
    }

    #[derive(Deserialize)]
    struct SavedObject {
        OBJECTNO: u8,
        OROOM: u8,
        OPIC: u8,
        OXCOORD: u8,
        OYCOORD: u8,
        OSTATUS: u8,
    }

    #[derive(Deserialize)]
    struct SavedRoom {
        RROOMNO: u8,
        RBASEPIC: u8,
        RCURPIC: u8,
    }

    #[derive(Deserialize)]
    struct Loader {
        menu: Vec<String>,
        pages: Vec<Vec<String>>,
    }

    fn numbered<T>(map: HashMap<String, T>, what: &str) -> Result<HashMap<u8, T>, String> {
        map.into_iter()
            .map(|(k, v)| {
                k.parse()
                    .map(|n| (n, v))
                    .map_err(|_| format!("{what} {k:?}"))
            })
            .collect()
    }

    fn condition(op: &Op) -> Result<Condition, String> {
        let a = &op.args;
        let arg = |i: usize| {
            a.get(i)
                .copied()
                .ok_or_else(|| format!("{} needs {} args", op.op, i + 1))
        };
        Ok(match op.op.as_str() {
            "OBJ_IN_ROOM" => Condition::ObjInRoom(arg(0)?, arg(1)?),
            "TURNS_GT" => Condition::TurnsGt(arg(0)?),
            "SWITCH_EQ" => Condition::SwitchEq(arg(0)?, arg(1)?),
            "VIEW_IS" => Condition::ViewIs(arg(0)?),
            "OBJ_PIC_IS" => Condition::ObjPicIs(arg(0)?, arg(1)?),
            other => return Err(format!("unknown condition {other}")),
        })
    }

    fn action(op: &Op) -> Result<Action, String> {
        let a = &op.args;
        let arg = |i: usize| {
            a.get(i)
                .copied()
                .ok_or_else(|| format!("{} needs {} args", op.op, i + 1))
        };
        Ok(match op.op.as_str() {
            "ADD_SWITCH" => Action::AddSwitch(arg(0)?, arg(1)?),
            "SUB_SWITCH" => Action::SubSwitch(arg(0)?, arg(1)?),
            "SET_SWITCH" => Action::SetSwitch(arg(0)?, arg(1)?),
            "INVENTORY" => Action::Inventory,
            "PUT_OBJ" => Action::PutObj(arg(0)?, arg(1)?),
            "PLACE_OBJ" => Action::PlaceObj(arg(0)?, arg(1)?, arg(2)?, arg(3)?),
            "GOTO_ROOM" => Action::GotoRoom(arg(0)?),
            "SET_VIEW" => Action::SetView(arg(0)?),
            "SET_BASE_VIEW" => Action::SetBaseView(arg(0)?),
            "RESET_VIEW" => Action::ResetView,
            "SET_ROOM_VIEW" => Action::SetRoomView(arg(0)?, arg(1)?),
            "SET_OBJ_PIC" => Action::SetObjPic(arg(0)?, arg(1)?),
            "MESSAGE" => Action::Message(arg(0)?),
            "LAMP_ON" => Action::LampOn,
            "LAMP_OFF" => Action::LampOff,
            "GO_N" => Action::Go(0),
            "GO_S" => Action::Go(1),
            "GO_E" => Action::Go(2),
            "GO_W" => Action::Go(3),
            "GO_U" => Action::Go(4),
            "GO_D" => Action::Go(5),
            "GET" => Action::Get,
            "DROP" => Action::Drop,
            "SAVE_GAME" => Action::SaveGame,
            "RESTORE_GAME" => Action::RestoreGame,
            "GAME_OVER" => Action::GameOver,
            "QUIT" => Action::Quit,
            other => return Err(format!("unknown action {other}")),
        })
    }

    fn rules(rules: Vec<Rule>) -> Result<Vec<super::Rule>, String> {
        rules
            .into_iter()
            .map(|r| {
                Ok(super::Rule {
                    addr: r.addr,
                    room: r.room,
                    verb: r.verb,
                    noun: r.noun,
                    conditions: r
                        .conditions
                        .iter()
                        .map(condition)
                        .collect::<Result<_, _>>()?,
                    actions: r.actions.iter().map(action).collect::<Result<_, _>>()?,
                })
            })
            .collect()
    }

    impl GameData {
        pub fn convert(self) -> Result<super::GameData, String> {
            for (i, r) in self.rooms.iter().enumerate() {
                if r.RROOMNO as usize != i + 1 {
                    return Err(format!("room {} is at position {}", r.RROOMNO, i + 1));
                }
            }
            let pictures = numbered(self.pictures, "picture")?
                .into_iter()
                .map(|(n, p)| {
                    let strokes = p
                        .strokes
                        .into_iter()
                        .map(|s| s.into_iter().map(|[x, y]| (x, y)).collect())
                        .collect();
                    (
                        n,
                        super::Picture {
                            continues: p.continues,
                            strokes,
                        },
                    )
                })
                .collect();
            let shapes = numbered(self.shapes, "shape")?
                .into_iter()
                .map(|(n, v)| (n, v.into_iter().map(|[p, d]| (p != 0, d)).collect()))
                .collect();
            let s = self.saved_game;
            Ok(super::GameData {
                verbs: self.verbs,
                nouns: self.nouns,
                messages: numbered(self.messages, "message")?,
                rooms: self
                    .rooms
                    .into_iter()
                    .map(|r| super::Room {
                        number: r.RROOMNO,
                        message: r.RMSG,
                        exits: [r.RN, r.RS, r.RE, r.RW, r.RU, r.RD],
                        base_picture: r.RBASEPIC,
                        current_picture: r.RCURPIC,
                    })
                    .collect(),
                objects: self
                    .objects
                    .into_iter()
                    .map(|o| super::Object {
                        id: o.OBJECTNO,
                        noun: o.noun,
                        room: o.OROOM,
                        picture: o.OPIC,
                        shape: o.shape,
                        x: o.OXCOORD,
                        y: o.OYCOORD,
                        status: o.OSTATUS,
                        message: o.OMSG,
                        in_pictures: o.ORPICSTA,
                    })
                    .collect(),
                pictures,
                shapes,
                command_rules: rules(self.command_rules)?,
                turn_rules: rules(self.turn_rules)?,
                floor_slots: self.floor_slots.into_iter().map(|[x, y]| (x, y)).collect(),
                saved_game: super::SavedGame {
                    room: s.room,
                    turn: s.turn,
                    switches: s.switches,
                    light: s.light,
                    verb: s.verb,
                    noun: s.noun,
                    objects: s
                        .objects
                        .into_iter()
                        .map(|o| (o.OBJECTNO, o.OROOM, o.OPIC, o.OXCOORD, o.OYCOORD, o.OSTATUS))
                        .collect(),
                    rooms: s
                        .rooms
                        .into_iter()
                        .map(|r| (r.RROOMNO, r.RBASEPIC, r.RCURPIC))
                        .collect(),
                },
                loader: super::Loader {
                    menu: self.loader.menu,
                    pages: self.loader.pages,
                },
            })
        }
    }
}
