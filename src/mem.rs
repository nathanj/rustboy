use std::cell::RefCell;
use std::rc::Rc;

use lcd;

pub struct MemoryMap {
    pub rom: Vec<u8>,
    pub vram: [u8; 0x2000],
    pub wram: [u8; 0x2000],
    pub hram: [u8; 0x80],
    pub interrupt_enable : bool,
    pub interrupt_master_enable : bool,
    pub interrupt_flag : u8,
    pub lcd : Rc<RefCell<lcd::Lcd>>,
}

impl MemoryMap {
    fn ioport_write(&mut self, addr: u16, write: bool, val: u8) -> u8 {
        match addr {
            0xff44 => {
                if write {
                    self.lcd.borrow_mut().ly = val;
                }
                self.lcd.borrow().ly
            }
            _ => panic!("bad addr {:04x}", addr),
        }
    }

    fn mmap_get_addr(&mut self, addr: u16, write: bool, val: u8) -> u8 {
        match addr {
            // rom bank 0
            0 ... 0x3fff => {
                if write {
                    self.rom[addr as usize] = val;
                }
                self.rom[addr as usize]
            },
            // rom bank n
            0x4000 ... 0x7fff => {
                if write {
                    self.rom[addr as usize] = val;
                }
                self.rom[addr as usize]
            },
            // vram
            0x8000 ... 0x9fff => {
                if write {
                    self.vram[addr as usize - 0x8000] = val;
                }
                self.vram[addr as usize - 0x8000]
            },
            // wram
            0xc000 ... 0xe000 => {
                if write {
                    self.wram[addr as usize - 0xc000] = val;
                }
                self.wram[addr as usize - 0xc000]
            },
            // hram
            0xff80 ... 0xfffe => {
                if write {
                    self.hram[addr as usize - 0xff80] = val;
                }
                self.hram[addr as usize - 0xff80]
            },
            // ioports
            0xff00 ... 0xff7f => {
                self.ioport_write(addr, write, 23)
            },
            _ => {
                // TODO
                panic!("mmap_get_addr() bad addr {:04x}", addr);
            }
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        self.mmap_get_addr(addr, true, val);
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        self.mmap_get_addr(addr, false, 0)
    }
}
