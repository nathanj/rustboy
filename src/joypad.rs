use sdl2::keyboard::Keycode;

use mem;
use interrupt;

#[derive(Debug)]
pub struct Joypad {
    pub flags : u8,
    up : bool,
    down : bool,
    left : bool,
    right : bool,
    b : bool,
    a : bool,
    select : bool,
    start : bool,
}

const JOYPAD_SELECT_BUTTON_KEYS    : u8 = 1<<5;
const JOYPAD_SELECT_DIRECTION_KEYS : u8 = 1<<4;

const JOYPAD_INPUT_DOWN            : u8 = 1<<3;
const JOYPAD_INPUT_UP              : u8 = 1<<2;
const JOYPAD_INPUT_LEFT            : u8 = 1<<1;
const JOYPAD_INPUT_RIGHT           : u8 = 1<<0;

const JOYPAD_INPUT_START           : u8 = 1<<3;
const JOYPAD_INPUT_SELECT          : u8 = 1<<2;
const JOYPAD_INPUT_BUTTON_B        : u8 = 1<<1;
const JOYPAD_INPUT_BUTTON_A        : u8 = 1<<0;

impl Joypad {

    pub fn new() -> Joypad {
        Joypad {
            flags: 0xff,
            up : false,
            down : false,
            left : false,
            right : false,
            b : false,
            a : false,
            select : false,
            start : false,
        }
    }

    pub fn set_flags(&mut self) {
        //println!("{:?}", self);
        self.flags |= 0x0f;
        if self.flags & JOYPAD_SELECT_DIRECTION_KEYS == 0 {
            if self.up { self.flags &= !JOYPAD_INPUT_UP; }
            if self.down { self.flags &= !JOYPAD_INPUT_DOWN; }
            if self.left { self.flags &= !JOYPAD_INPUT_LEFT; }
            if self.right { self.flags &= !JOYPAD_INPUT_RIGHT; }
        }
        if self.flags & JOYPAD_SELECT_BUTTON_KEYS == 0 {
            if self.b { self.flags &= !JOYPAD_INPUT_BUTTON_B; }
            if self.a { self.flags &= !JOYPAD_INPUT_BUTTON_A; }
            if self.select { self.flags &= !JOYPAD_INPUT_SELECT; }
            if self.start { self.flags &= !JOYPAD_INPUT_START }
        }
        //println!("flags = {:02x}", self.flags);
    }

    pub fn handle_input(&mut self, mm: &mut mem::MemoryMap, keycode: Keycode, pressed: bool) {
        //println!("keycode={} pressed={}", keycode, pressed);

        match keycode {
            Keycode::Up => { self.up = pressed; }
            Keycode::Down => { self.down = pressed; }
            Keycode::Left => { self.left = pressed; }
            Keycode::Right => { self.right = pressed; }
            Keycode::Z => { self.b = pressed; }
            Keycode::X => { self.a = pressed; }
            Keycode::A => { self.select = pressed; }
            Keycode::S => { self.start = pressed; }
            Keycode::B => {
                self.start = pressed;
                self.b = pressed;
                self.a = pressed;
                self.select = pressed;
            }
            _ => {}
        }

        if pressed {
            match keycode {
                Keycode::L => { mm.dump(0xc000, 8*32); }
                Keycode::O => { mm.dump(0xfe00, 0xa0); }
                _ => {}
            }
        }

        self.set_flags();

        if mm.interrupt_master_enable {
            mm.interrupt_flag |= interrupt::INTERRUPT_JOYPAD;
        }
    }

}

#[test]
fn test_joypad() {
    let joypad = Joypad::new();
    assert_eq!(joypad.flags, 0xff);
}
