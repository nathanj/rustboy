use sdl2::keyboard::Keycode;

use mem;
use interrupt;

pub struct Joypad {
    pub flags : u8,
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
        }
    }

    pub fn handle_input(&mut self, mm: &mut mem::MemoryMap, keycode: Keycode, pressed: bool) {
        self.flags |= 0x0f;

        println!("keycode={} pressed={}", keycode, pressed);

        if (self.flags & JOYPAD_SELECT_DIRECTION_KEYS) == 0 {
            match keycode {
                Keycode::Up => { self.flags &= !JOYPAD_INPUT_UP }
                Keycode::Down => { self.flags &= !JOYPAD_INPUT_DOWN }
                Keycode::Left => { self.flags &= !JOYPAD_INPUT_LEFT }
                Keycode::Right => { self.flags &= !JOYPAD_INPUT_RIGHT }
                _ => {}
            }
        }
        if (self.flags & JOYPAD_SELECT_BUTTON_KEYS) == 0 {
            match keycode {
                Keycode::Z => { self.flags &= !JOYPAD_INPUT_BUTTON_B }
                Keycode::X => { self.flags &= !JOYPAD_INPUT_BUTTON_A }
                Keycode::A => { self.flags &= !JOYPAD_INPUT_SELECT }
                Keycode::S => { self.flags &= !JOYPAD_INPUT_START }
                _ => {}
            }
        }

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
