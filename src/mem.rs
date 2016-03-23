use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::RwLock;

use lcd;
use timer;
use joypad;
use sound;

pub struct MemoryMap {
    pub rom: Vec<u8>,
    pub vram: [u8; 0x2000],
    pub wram: [u8; 0x2000],
    pub hram: [u8; 0x80],
    pub iobuf: [u8; 0x100],
    pub oam: [u8; 0xa0],
    pub interrupt_enable : u8,
    pub interrupt_master_enable : bool,
    pub interrupt_flag : u8,
    pub lcd : Rc<RefCell<lcd::Lcd>>,
    pub timer : Rc<RefCell<timer::Timer>>,
    pub joypad : Rc<RefCell<joypad::Joypad>>,
    pub sound : Arc<RwLock<sound::Sound>>,
    pub rom_bank: u8,
}

impl MemoryMap {
    fn perform_dma(&mut self, val: u8) {
        for i in 0..0xa0 {
            let val = self.read(val as u16 * 0x100 + i);
            self.oam[i as usize] = val;
        }
    }

    fn handle_ioport(&mut self, addr: u16, write: bool, val: u8) -> u8 {
        match addr {
            0xff00 => {
                if write {
                    let mut joypad = self.joypad.borrow_mut();
                    joypad.flags = val;
                    joypad.set_flags();
                }
                self.joypad.borrow().flags
            }
            0xff01 => { 0 } // serial_transfer_data
            0xff02 => { 0 } // serial_transfer_control
            0xff04 => { if write { self.timer.borrow_mut().div = val; } self.timer.borrow().div }
            0xff05 => { if write { self.timer.borrow_mut().tima = val; } self.timer.borrow().tima }
            0xff06 => { if write { self.timer.borrow_mut().tma = val; } self.timer.borrow().tma }
            0xff07 => { if write { self.timer.borrow_mut().tac = val; } self.timer.borrow().tac }

            0xff10 ... 0xff3f => { self.sound.write().unwrap().handle_addr(addr, write, val) }

            0xff40 => { if write { self.lcd.borrow_mut().ctl = val; } self.lcd.borrow().ctl }
            0xff41 => { if write { self.lcd.borrow_mut().stat = val; } self.lcd.borrow().stat }
            0xff42 => { if write { self.lcd.borrow_mut().scy = val; } self.lcd.borrow().scy }
            0xff43 => { if write { self.lcd.borrow_mut().scx = val; } self.lcd.borrow().scx }
            0xff44 => { if write { self.lcd.borrow_mut().ly = val; } self.lcd.borrow().ly }
            0xff45 => { if write { self.lcd.borrow_mut().lyc = val; } self.lcd.borrow().lyc }
            0xff46 => { if write { self.perform_dma(val); } 0 }
            0xff47 => { if write { self.lcd.borrow_mut().bgp = val; } self.lcd.borrow().bgp }
            0xff48 => { if write { self.lcd.borrow_mut().obp0 = val; } self.lcd.borrow().obp0 }
            0xff49 => { if write { self.lcd.borrow_mut().obp1 = val; } self.lcd.borrow().obp1 }
            0xff4a => { if write { self.lcd.borrow_mut().wy = val; } self.lcd.borrow().wy }
            0xff4b => { if write { self.lcd.borrow_mut().wx = val; } self.lcd.borrow().wx }
            0xff0f => { if write { self.interrupt_flag = val; } self.interrupt_flag }
            0xffff => { if write { self.interrupt_enable = val; } self.interrupt_enable }
            _ => {
                // if write {
                //     self.iobuf[addr as usize - 0xff00] = val;
                // }
                // self.iobuf[addr as usize - 0xff00]
                0
            }
        }
    }

    fn handle_addr(&mut self, addr: u16, write: bool, val: u8) -> u8 {
        match addr {
            // rom bank 0
            0 ... 0x1fff => {
                if write {
                    println!("ram enable {:02x}", val);
                }
                self.rom[addr as usize]
            },
            0x2000 ... 0x3fff => {
                if write {
                    println!("rom bank number {:02x}", val);
                    self.rom_bank = val;
                }
                self.rom[addr as usize]
            },
            // rom bank n
            0x4000 ... 0x5fff => {
                if write {
                    println!("ram bank number {:02x}", val);
                }
                self.rom[self.rom_bank as usize * 0x4000 + (addr - 0x4000) as usize]
            },
            0x6000 ... 0x7fff => {
                if write {
                    println!("rom/ram mode select {:02x}", val);
                }
                self.rom[self.rom_bank as usize * 0x4000 + (addr - 0x4000) as usize]
            },
            // vram
            0x8000 ... 0x9fff => {
                if write {
                    self.vram[addr as usize - 0x8000] = val;
                }
                self.vram[addr as usize - 0x8000]
            },
            // wram
            0xc000 ... 0xdfff => {
                if write {
                    self.wram[addr as usize - 0xc000] = val;
                }
                self.wram[addr as usize - 0xc000]
            },
            // wram bank 0 echo
            0xe000 ... 0xfdff => {
                if write {
                    self.wram[addr as usize - 0xe000] = val;
                }
                self.wram[addr as usize - 0xe000]
            },
            // oam
            0xfe00 ... 0xfe9f => {
                if write {
                    self.oam[addr as usize - 0xfe00] = val;
                }
                self.oam[addr as usize - 0xfe00]
            },
            // not usable area
            0xfea0 ... 0xfeff => {
                trace!("not usable {:04x}", addr);
                0
            }
            // ioports
            0xff00 ... 0xff7f => {
                self.handle_ioport(addr, write, val)
            },
            // hram
            0xff80 ... 0xfffe => {
                if write {
                    self.hram[addr as usize - 0xff80] = val;
                }
                self.hram[addr as usize - 0xff80]
            },
            // interrupt_enable
            0xffff => {
                if write {
                    self.interrupt_enable = val;
                }
                self.interrupt_enable
            }
            _ => {
                0
                // TODO
                //panic!("handle_addr() bad addr {:04x}", addr);
            }
        }
    }

    pub fn dump(&mut self, start: u16, len: u16) {
        for x in 0..len/32 {
            println!("{:04x}: {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}  {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}",
                     start + x * 32,
                     self.read(start + x * 32 + 0),
                     self.read(start + x * 32 + 1),
                     self.read(start + x * 32 + 2),
                     self.read(start + x * 32 + 3),
                     self.read(start + x * 32 + 4),
                     self.read(start + x * 32 + 5),
                     self.read(start + x * 32 + 6),
                     self.read(start + x * 32 + 7),
                     self.read(start + x * 32 + 8),
                     self.read(start + x * 32 + 9),
                     self.read(start + x * 32 + 10),
                     self.read(start + x * 32 + 11),
                     self.read(start + x * 32 + 12),
                     self.read(start + x * 32 + 13),
                     self.read(start + x * 32 + 14),
                     self.read(start + x * 32 + 15),
                     self.read(start + x * 32 + 16),
                     self.read(start + x * 32 + 17),
                     self.read(start + x * 32 + 18),
                     self.read(start + x * 32 + 19),
                     self.read(start + x * 32 + 20),
                     self.read(start + x * 32 + 21),
                     self.read(start + x * 32 + 22),
                     self.read(start + x * 32 + 23),
                     self.read(start + x * 32 + 24),
                     self.read(start + x * 32 + 25),
                     self.read(start + x * 32 + 26),
                     self.read(start + x * 32 + 27),
                     self.read(start + x * 32 + 28),
                     self.read(start + x * 32 + 29),
                     self.read(start + x * 32 + 30),
                     self.read(start + x * 32 + 31));
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        self.handle_addr(addr, true, val);
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        self.handle_addr(addr, false, 0)
    }

    pub fn di(&mut self) {
        self.interrupt_master_enable = false;
    }

    pub fn ei(&mut self) {
        self.interrupt_master_enable = true;
    }

    pub fn interrupt_triggered(&mut self, interrupt: u8) -> bool {
        if !self.interrupt_master_enable {
            return false;
        }

        let triggered = self.interrupt_enable & interrupt > 0 && self.interrupt_flag & interrupt > 0;
        if triggered {
            self.interrupt_master_enable = false;
            self.interrupt_flag &= !interrupt;
        }
        return triggered;
    }
}
