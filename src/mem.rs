use std::cell::RefCell;
use std::rc::Rc;

use lcd;

pub struct MemoryMap {
    pub rom: Vec<u8>,
    pub vram: [u8; 0x2000],
    pub wram: [u8; 0x2000],
    pub hram: [u8; 0x80],
    pub iobuf: [u8; 0x100],
    pub oam: [u8; 40],
    pub interrupt_enable : u8,
    pub interrupt_master_enable : bool,
    pub interrupt_flag : u8,
    pub lcd : Rc<RefCell<lcd::Lcd>>,
}

impl MemoryMap {
    fn handle_ioport(&mut self, addr: u16, write: bool, val: u8) -> u8 {
        match addr {
            // 0xff00 => joypad
            0xff01 => { 0 } // serial_transfer_data
            0xff02 => { 0 } // serial_transfer_control
            // 0xff04 => timer.div
            // 0xff05 => timer.tima
            // 0xff06 => timer.tma
            // 0xff07 => timer.tac
            0xff40 => { if write { self.lcd.borrow_mut().ctl = val; } self.lcd.borrow().ctl }
            0xff41 => { if write { self.lcd.borrow_mut().stat = val; } self.lcd.borrow().stat }
            0xff42 => { if write { self.lcd.borrow_mut().scy = val; } self.lcd.borrow().scy }
            0xff43 => { if write { self.lcd.borrow_mut().scx = val; } self.lcd.borrow().scx }
            0xff44 => { if write { self.lcd.borrow_mut().ly = val; } self.lcd.borrow().ly }
            0xff45 => { if write { self.lcd.borrow_mut().lyc = val; } self.lcd.borrow().lyc }
            0xff46 => { if write { self.lcd.borrow_mut().dma = val; } self.lcd.borrow().dma }
            0xff47 => { if write { self.lcd.borrow_mut().bgp = val; } self.lcd.borrow().bgp }
            0xff48 => { if write { self.lcd.borrow_mut().obp0 = val; } self.lcd.borrow().obp0 }
            0xff49 => { if write { self.lcd.borrow_mut().obp1 = val; } self.lcd.borrow().obp1 }
            0xff4a => { if write { self.lcd.borrow_mut().wy = val; } self.lcd.borrow().wy }
            0xff4b => { if write { self.lcd.borrow_mut().wx = val; } self.lcd.borrow().wx }
            0xff0f => { if write { self.interrupt_flag = val; } self.interrupt_flag }
            0xffff => { if write { self.interrupt_enable = val; } self.interrupt_enable }
            _ => {
                if write {
                    self.iobuf[addr as usize - 0xff00] = val;
                }
                self.iobuf[addr as usize - 0xff00]
            }
        }
    }

    fn handle_addr(&mut self, addr: u16, write: bool, val: u8) -> u8 {
        match addr {
            // rom bank 0
            0 ... 0x3fff => {
                if write {
                    panic!("bad write at {:04x}", addr);
                }
                self.rom[addr as usize]
            },
            // rom bank n
            0x4000 ... 0x7fff => {
                if write {
                    panic!("bad write at {:04x}", addr);
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
            // oam
            0xfe00 ... 0xfe9f => {
                if write {
                    self.oam[addr as usize - 0xfe00] = val;
                }
                self.oam[addr as usize - 0xfe00]
            },
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
                // TODO
                panic!("handle_addr() bad addr {:04x}", addr);
            }
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
}
