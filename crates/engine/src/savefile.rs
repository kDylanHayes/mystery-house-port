//! The original's `SAVED GAME` file: the 2255 bytes at `$0830-$10FE` that `SAVE GAME` BSAVEs
//! and `RESTORE GAME` BLOADs (decomp/PORTING.md, "Saving").
//!
//! A file written here loads in the original too. Besides the state, that needs the variables
//! the game reads after a restore before writing them: `blank_word` (eight spaces, copied into
//! every word the parser reads), the drawing offsets at 0, and `cmp_result` and `actions_left`
//! at 1, as they are while the `SAVE GAME` rule runs. Everything else in the range is scratch
//! or leftover memory, written as 0.

use crate::data::GameData;
use crate::{ObjectState, RoomState, State};

pub const START: u16 = 0x0830;
pub const LEN: usize = 0x10FF - 0x0830;

// Variables that the exported addresses do not name, from the layout in src/adventure.s.
const DRAW_DY: u16 = 0x084E;
const DRAW_DX: u16 = 0x084F;
const PEN_UP: u16 = 0x0850;
const CMP_RESULT: u16 = 0x0851;
const BLANK_WORD: u16 = 0x0875;
const ACTIONS_LEFT: u16 = 0x0898;
const ROOM_RECORD: u16 = 10;

fn addr(data: &GameData, name: &str) -> u16 {
    *data
        .addresses
        .get(name)
        .unwrap_or_else(|| panic!("gamedata.json has no address {name}"))
}

/// The save file for a state.
pub fn write(data: &GameData, s: &State) -> Vec<u8> {
    let mut mem = vec![0u8; LEN];
    let mut put = |a: u16, b: u8| mem[(a - START) as usize] = b;

    put(addr(data, "CURONO"), s.room);
    let turns = addr(data, "TURNS");
    put(turns, s.turn as u8);
    put(turns + 1, (s.turn >> 8) as u8);
    let switch = addr(data, "SWITCH");
    for (i, &v) in s.switches.iter().enumerate() {
        put(switch + i as u16, v);
    }
    put(addr(data, "LAMPSTAT"), if s.lamp_on { 0 } else { 1 });
    put(addr(data, "VERB"), s.verb);
    put(addr(data, "NOUN"), s.noun);
    put(addr(data, "SHOWN_PICTURE"), s.shown_picture);
    put(PEN_UP, s.pen_up as u8);
    put(DRAW_DX, 0);
    put(DRAW_DY, 0);
    put(CMP_RESULT, 1);
    put(ACTIONS_LEFT, 1);
    for i in 0..8 {
        put(BLANK_WORD + i, 0xA0);
    }

    // The object table: every record as the game keeps it, then the end mark.
    let mut end = addr(data, "OBJBASE");
    for (o, st) in data.objects.iter().zip(&s.objects) {
        let pics = &o.in_pictures;
        let record = [
            o.id,
            o.noun,
            st.room,
            st.picture,
            o.shape as u8,
            st.x,
            st.y,
            st.status,
            o.message,
            11 + pics.len() as u8,
            pics.len() as u8,
        ];
        for (i, &b) in record.iter().chain(pics).enumerate() {
            put(o.addr + i as u16, b);
        }
        end = o.addr + 11 + pics.len() as u16;
    }
    put(end, 0xFF);

    // The room table, indexed by room number (room 0 unused).
    let base = addr(data, "ROOMBASE");
    for (r, st) in data.rooms.iter().zip(&s.rooms) {
        let a = base + ROOM_RECORD * r.number as u16;
        let [n, sth, e, w, u, d] = r.exits;
        let record = [
            r.number,
            r.message,
            n,
            sth,
            e,
            w,
            u,
            d,
            st.base_picture,
            st.current_picture,
        ];
        for (i, &b) in record.iter().enumerate() {
            put(a + i as u16, b);
        }
    }
    mem
}

/// The state in a save file. Takes the 2255 bytes, or a DOS 3.3 binary file with its 4-byte
/// header (load address $0830, length $08CF) in front.
pub fn read(data: &GameData, file: &[u8]) -> Result<State, String> {
    let mem = match file.len() {
        LEN => file,
        n if n == LEN + 4 && file[..4] == [0x30, 0x08, 0xCF, 0x08] => &file[4..],
        n => return Err(format!("a saved game is {LEN} bytes, not {n}")),
    };
    let at = |a: u16| mem[(a - START) as usize];
    let turns = addr(data, "TURNS");
    let switch = addr(data, "SWITCH");
    let mut objects = Vec::with_capacity(data.objects.len());
    for o in &data.objects {
        if at(o.addr) != o.id {
            return Err(format!(
                "no object {} at ${:04X}: not a Mystery House save",
                o.id, o.addr
            ));
        }
        objects.push(ObjectState {
            room: at(o.addr + 2),
            picture: at(o.addr + 3),
            x: at(o.addr + 5),
            y: at(o.addr + 6),
            status: at(o.addr + 7),
        });
    }
    let base = addr(data, "ROOMBASE");
    let rooms = data
        .rooms
        .iter()
        .map(|r| {
            let a = base + ROOM_RECORD * r.number as u16;
            RoomState {
                base_picture: at(a + 8),
                current_picture: at(a + 9),
            }
        })
        .collect();
    let room = at(addr(data, "CURONO"));
    if room == 0 || room as usize > data.rooms.len() {
        return Err(format!("room {room} does not exist"));
    }
    Ok(State {
        room,
        turn: at(turns) as u16 | (at(turns + 1) as u16) << 8,
        switches: std::array::from_fn(|i| at(switch + i as u16)),
        lamp_on: at(addr(data, "LAMPSTAT")) == 0,
        verb: at(addr(data, "VERB")),
        noun: at(addr(data, "NOUN")),
        shown_picture: at(addr(data, "SHOWN_PICTURE")),
        pen_up: at(PEN_UP) != 0,
        objects,
        rooms,
    })
}
