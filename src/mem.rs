use std::cell::RefCell;
use std::rc::Rc;

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
    pub sound : Rc<RefCell<sound::Sound>>,
}

impl MemoryMap {
    fn perform_dma(&mut self, val: u8) {
        //println!("performing dma");
        for i in 0..0xa0 {
            let val = self.read(val as u16 * 0x100 + i);
            self.oam[i as usize] = val;
            if i >= 0x15 && i <= 0x20{
                //println!("writing oam {:04x} = {:02x}", 0xfe00 + i, val);
            }
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

            0xff10 => { if write { self.sound.borrow_mut().nr10 = val; } self.sound.borrow().nr10 }
            0xff11 => { if write { self.sound.borrow_mut().nr11 = val; } self.sound.borrow().nr11 }
            0xff12 => { if write { self.sound.borrow_mut().nr12 = val; } self.sound.borrow().nr12 }
            0xff13 => { if write { self.sound.borrow_mut().nr13 = val; } self.sound.borrow().nr13 }
            0xff14 => { if write { self.sound.borrow_mut().nr14 = val; } self.sound.borrow().nr14 }
            0xff16 => { if write { self.sound.borrow_mut().nr21 = val; } self.sound.borrow().nr21 }
            0xff17 => { if write { self.sound.borrow_mut().nr22 = val; } self.sound.borrow().nr22 }
            0xff18 => { if write { self.sound.borrow_mut().nr23 = val; } self.sound.borrow().nr23 }
            0xff19 => { if write { self.sound.borrow_mut().nr24 = val; } self.sound.borrow().nr24 }
            0xff1a => { if write { self.sound.borrow_mut().nr30 = val; } self.sound.borrow().nr30 }
            0xff1b => { if write { self.sound.borrow_mut().nr31 = val; } self.sound.borrow().nr31 }
            0xff1c => { if write { self.sound.borrow_mut().nr32 = val; } self.sound.borrow().nr32 }
            0xff1d => { if write { self.sound.borrow_mut().nr33 = val; } self.sound.borrow().nr33 }
            0xff1e => { if write { self.sound.borrow_mut().nr34 = val; } self.sound.borrow().nr34 }
            0xff20 => { if write { self.sound.borrow_mut().nr41 = val; } self.sound.borrow().nr41 }
            0xff21 => { if write { self.sound.borrow_mut().nr42 = val; } self.sound.borrow().nr42 }
            0xff22 => { if write { self.sound.borrow_mut().nr43 = val; } self.sound.borrow().nr43 }
            0xff23 => { if write { self.sound.borrow_mut().nr44 = val; } self.sound.borrow().nr44 }
            0xff24 => { if write { self.sound.borrow_mut().nr50 = val; } self.sound.borrow().nr50 }
            0xff25 => { if write { self.sound.borrow_mut().nr51 = val; } self.sound.borrow().nr51 }
            0xff26 => { if write { self.sound.borrow_mut().nr52 = val; } self.sound.borrow().nr52 }

            //0xff30 ... 0xff3f => { if write { self.sound.borrow_mut().wave_ram[addr - 0xff30 as usize] = val; } self.sound.borrow().wave_ram[addr - 0xff30 as usize] }

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
        //if write && (addr == 0x9950 || addr == 0xdf71) {
        //    println!("NJJ writing addr={:04x} val={:02x}", addr, val);
        //}
        //if write && (addr == 0xffe1 || addr == 0xffe2 || addr==0xcfec || addr==0xcfed || addr == 0xcfee || addr == 0xcfef) {
        //    println!("NJJ writing addr={:04x} val={:02x}", addr, val);
        //}
        if write && (addr >= 0xc000 && addr < 0xc050) {
            println!("NJJ writing addr={:04x} val={:02x}", addr, val);
        }
        if write && (addr == 0xff87 || addr == 0xff8b || addr == 0xff90 || addr == 0xc201 || addr == 0xc202 || addr == 0xc203 || addr == 0xffea) {
            println!("NJJ writing addr={:04x} val={:02x}", addr, val);
        }
        //if write && (addr >= 0xff80 || addr == 0xc201 || addr == 0xc202 || addr == 0xcff5 || addr == 0xcff6 || addr == 0xcff7) {
        //    println!("NJJ writing addr={:04x} val={:02x}", addr, val);
        //}
        match addr {
            // rom bank 0
            0 ... 0x3fff => {
                if write {
                    trace!("bad write at {:04x}", addr);
                }
                self.rom[addr as usize]
            },
            // rom bank n
            0x4000 ... 0x7fff => {
                if write {
                    trace!("bad write at {:04x}", addr);
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
            0xc000 ... 0xdfff => {
                if write {
                    if addr == 0xc019 {
                        println!("writing {:04x} = {:02x}", addr, val);
                    }
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
                    trace!("writing oam {:04x} = {:02x}", addr, val);
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
                    if addr == 0xffe1 && val == 0 {
                        self.dump(0x8000, 0xfff0 - 0x8000);
                    }
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
        for x in 0..len/16 {
            println!("{:04x}: {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}",
                     start + x * 16,
                     self.read(start + x * 16 + 0),
                     self.read(start + x * 16 + 1),
                     self.read(start + x * 16 + 2),
                     self.read(start + x * 16 + 3),
                     self.read(start + x * 16 + 4),
                     self.read(start + x * 16 + 5),
                     self.read(start + x * 16 + 6),
                     self.read(start + x * 16 + 7),
                     self.read(start + x * 16 + 8),
                     self.read(start + x * 16 + 9),
                     self.read(start + x * 16 + 10),
                     self.read(start + x * 16 + 11),
                     self.read(start + x * 16 + 12),
                     self.read(start + x * 16 + 13),
                     self.read(start + x * 16 + 14),
                     self.read(start + x * 16 + 15));
        }
    }

    pub fn dump_oam(&self) {
        for x in 0..20 {
            println!("{:02x}: {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}",
                     0xfe00 + x * 8,
                     self.oam[x*8+0], self.oam[x*8+1], self.oam[x*8+2], self.oam[x*8+3],
                     self.oam[x*8+4], self.oam[x*8+5], self.oam[x*8+6], self.oam[x*8+7]);
        }
    }

    pub fn dump_hram(&self) {
        for x in 0..0x10 {
            trace!("{:02x}: {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}",
                   0xff80 + x * 8,
                   self.hram[x*8+0], self.hram[x*8+1], self.hram[x*8+2], self.hram[x*8+3],
                   self.hram[x*8+4], self.hram[x*8+5], self.hram[x*8+6], self.hram[x*8+7]);
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
