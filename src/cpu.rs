use std::fmt;
use std::num;
use std::convert;
use std::cell::RefCell;
use std::rc::Rc;

use mem;
use lcd;
use timer;
use joypad;

pub struct Cpu {
    a: u8,
    f: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    pc: u16,
    sp: u16,
    cycles: u32,
}

impl fmt::Debug for Cpu {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Cpu {{ a:{:02x} f:{:02x} b:{:02x} c:{:02x} d:{:02x} \
               e:{:02x} h:{:02x} l:{:02x} pc:{:06x} sp:{:06x} cycles:{} }}",
               self.a, self.f, self.b, self.c, self.d, self.e, self.h, self.l,
               self.pc, self.sp, self.cycles)
    }
}

const FLAG_ZERO       : u8 = 0b1000_0000;
const FLAG_SUBTRACT   : u8 = 0b0100_0000;
const FLAG_HALF_CARRY : u8 = 0b0010_0000;
const FLAG_CARRY      : u8 = 0b0001_0000;

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            a: 0x01,
            f: 0xb0,
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xd8,
            h: 0x00,
            l: 0x00,
            sp: 0xfffe,
            pc: 0x100,
            cycles: 0,
        }
    }

    fn af(&self) -> u16 {
        return (self.a as u16) << 8 | (self.f as u16);
    }
    fn set_af(&mut self, af: u16) {
        self.a = (af >> 8) as u8;
        self.f = (af & 0xff) as u8;
    }
    fn bc(&self) -> u16 {
        return (self.b as u16) << 8 | (self.c as u16);
    }
    fn set_bc(&mut self, bc: u16) {
        self.b = (bc >> 8) as u8;
        self.c = (bc & 0xff) as u8;
    }
    fn de(&self) -> u16 {
        return (self.d as u16) << 8 | (self.e as u16);
    }
    fn set_de(&mut self, de: u16) {
        self.d = (de >> 8) as u8;
        self.e = (de & 0xff) as u8;
    }
    fn hl(&self) -> u16 {
        return (self.h as u16) << 8 | (self.l as u16);
    }
    fn set_hl(&mut self, hl: u16) {
        self.h = (hl >> 8) as u8;
        self.l = (hl & 0xff) as u8;
    }
    fn zero(&self) -> bool {
        return self.f & FLAG_ZERO > 0;
    }
    fn subtract(&self) -> bool {
        return self.f & FLAG_SUBTRACT > 0;
    }
    fn half_carry(&self) -> bool {
        return self.f & FLAG_HALF_CARRY > 0;
    }
    fn carry(&self) -> bool {
        return self.f & FLAG_CARRY > 0;
    }
    fn set_zero(&mut self, val: bool) {
        if val {
            self.f |= FLAG_ZERO;
        } else {
            self.f &= !FLAG_ZERO;
        }
    }
    fn set_subtract(&mut self, val: bool) {
        if val {
            self.f |= FLAG_SUBTRACT;
        } else {
            self.f &= !FLAG_SUBTRACT;
        }
    }
    fn set_half_carry(&mut self, val: bool) {
        if val {
            self.f |= FLAG_HALF_CARRY;
        } else {
            self.f &= !FLAG_HALF_CARRY;
        }
    }
    fn set_carry(&mut self, val: bool) {
        if val {
            self.f |= FLAG_CARRY;
        } else {
            self.f &= !FLAG_CARRY;
        }
    }

    fn read_u16(&self, mm: &mut mem::MemoryMap, pos: usize) -> u16 {
        let p = pos as u16;
        return (mm.read(p + 1) as u16) << 8 | (mm.read(p) as u16);
    }

    fn add(&mut self, val: u8) {
        let pa = self.a;
        self.a.wrapping_add(val);
        let a = self.a;
        self.set_zero(a == 0);
        self.set_subtract(false);
        self.set_half_carry(pa&0xf + val&0xf > 0xf);
        self.set_carry(a < pa);
    }

    fn adc(&mut self, val: u8) {
        let carry = if self.carry() { 1 } else { 0 };
        self.add(val + carry);
    }

    fn sub(&mut self, val: u8) {
        let pa = self.a;
        self.a.wrapping_sub(val);
        let a = self.a;
        self.set_zero(a == 0);
        self.set_subtract(true);
        // self.set_half_carry(pa&0xf + val&0xf > 0xf); // XXX
        self.set_carry(a > pa);
    }

    fn sbc(&mut self, val: u8) {
        let carry = if self.carry() { 1 } else { 0 };
        self.sub(val + carry);
    }

    fn and(&mut self, val: u8) {
        self.a &= val;
        let a = self.a;
        self.set_zero(a == 0);
        self.set_subtract(false);
        self.set_half_carry(true);
        self.set_carry(false);
    }

    fn xor(&mut self, val: u8) {
        self.a ^= val;
        let a = self.a;
        self.set_zero(a == 0);
        self.set_subtract(false);
        self.set_half_carry(false);
        self.set_carry(false);
    }

    fn or(&mut self, val: u8) {
        self.a |= val;
        let a = self.a;
        self.set_zero(a == 0);
        self.set_subtract(false);
        self.set_half_carry(false);
        self.set_carry(false);
    }

    fn cp(&mut self, val: u8) {
        let a = self.a;
        self.set_zero(a == val);
        //self.set_subtract(false);
        //self.set_half_carry(false);
        //self.set_carry(false);
    }

    fn rlc(&mut self, val: u8) -> u8 {
        let carry = val & 0x80;
        let mut newval = val << 1;

        println!("newval = {}", newval);

        if carry == 0 {
            self.set_carry(false);
        } else {
            newval |= 0x1;
            self.set_carry(true);
        }
        self.set_zero(newval == 0);

        return newval;
    }

    fn rl(&mut self, val: u8) -> u8 {
        let carry = val & 0x80;
        let mut newval = val << 1;

        if self.carry() {
            newval |= 0x1;
        }
        self.set_carry(carry != 0);
        self.set_zero(newval == 0);
        return newval;
    }

    fn sla(&mut self, val: u8) -> u8 {
        let carry = val & 0x80;
        let newval = val << 1;

        self.set_carry(carry != 0);
        self.set_zero(newval == 0);

        return newval;
    }

    fn sra(&mut self, val: u8) -> u8 {
        let carry = val & 0x1;
        let newval = val >> 1;

        self.set_carry(carry != 0);
        self.set_zero(newval == 0);

        return newval;
    }

    fn rrc(&mut self, val: u8) -> u8 {
        let carry = val & 0x1;
        let mut newval = val >> 1;

        if carry == 1 {
            newval |= 0x80;
            self.set_carry(true);
        } else {
            self.set_carry(false);
        }
        self.set_zero(newval == 0);

        return newval;
    }

    fn rr(&mut self, val: u8) -> u8 {
        let carry = val & 0x1;
        let mut newval = val >> 1;

        if self.carry() {
            newval |= 0x80;
        }
        self.set_carry(carry == 1);
        self.set_zero(newval == 0);
        return newval;
    }

    fn inc(&mut self, val: u8) -> u8 {
        let newval = val.wrapping_add(1);
        self.set_zero(newval == 0);
        self.set_subtract(false);
        self.set_half_carry(newval & 0xf == 0);
        return newval;
    }

    fn inc16(&mut self, val: u16) -> u16 {
        let newval = val.wrapping_add(1);
        self.set_zero(newval == 0);
        self.set_subtract(false);
        return newval;
    }

    fn dec(&mut self, val: u8) -> u8 {
        let newval = val.wrapping_sub(1);
        self.set_zero(newval == 0);
        self.set_subtract(true);
        self.set_half_carry(newval & 0xf == 0xf);
        return newval;
    }

    fn dec16(&mut self, val: u16) -> u16 {
        let newval = val.wrapping_sub(1);
        self.set_zero(newval == 0);
        self.set_subtract(true);
        return newval;
    }

    fn write_return_addr(&mut self, mm: &mut mem::MemoryMap, addr: u16) {
        mm.write(self.sp - 1, (addr >> 8) as u8);
        mm.write(self.sp - 2, (addr & 0xff) as u8);
        self.sp -= 2;
    }

    fn read_return_addr(&mut self, mm: &mut mem::MemoryMap) -> u16 {
        let lower = mm.read(self.sp);
        let upper = mm.read(self.sp + 1);
        self.sp += 2;
        return (upper as u16) << 8 | (lower as u16);
    }

    pub fn run(&mut self, mm: &mut mem::MemoryMap) -> u32 {
        let mut pc = self.pc as usize;
        trace!("{:?}", self);
        match mm.rom[pc] {
            0x00 => {
                trace!("nop");
                self.cycles += 4;
                pc += 1;
            },
            0x01 => {
                let val = self.read_u16(mm, pc + 1);
                trace!("ld bc, ${:04x}", val);
                self.set_bc(val);
                self.cycles += 12;
                pc += 3;
            },
            0x02 => {
                trace!("ld (bc), a");
                mm.write(self.bc(), self.a);
                self.cycles += 8;
                pc += 1;
            },
            0x03 => {
                trace!("inc bc");
                let bc = self.bc();
                let inc = self.inc16(bc);
                self.set_bc(inc);
                self.cycles += 8;
                pc += 1;
            },
            0x04 => {
                trace!("inc b");
                let b = self.b;
                self.b = self.inc(b);
                self.cycles += 4;
                pc += 1;
            },
            0x05 => {
                trace!("dec b");
                let b = self.b;
                self.b = self.dec(b);
                self.cycles += 4;
                pc += 1;
            },
            0x06 => {
                let val = mm.rom[pc + 1];
                trace!("ld b, ${:02x}", val);
                self.b = val;
                self.cycles += 8;
                pc += 2;
            },
            0x07 => {
                trace!("rlca");
                let val = self.a;
                self.a = self.rlc(val);
                self.cycles += 4;
                pc += 1;
            },
            0x08 => {
                let val = self.read_u16(mm, pc + 1);
                trace!("ld (${:04x}), sp", val);
                self.sp = val;
                self.cycles += 20;
                pc += 3;
            },
            0x09 => {
                trace!("add hl, bc");
                let bc = self.bc();
                let hl = self.hl();
                self.set_hl(hl.wrapping_add(bc));
                self.set_subtract(false);
                self.set_half_carry(false);
                self.set_carry(false /* TODO */);
                self.cycles += 8;
                pc += 1;
            },
            0x0a => {
                trace!("ld a, (bc)");
                self.a = mm.read(self.bc());
                self.cycles += 8;
                pc += 1;
            },
            0x0b => {
                trace!("dec bc");
                let bc = self.bc();
                let dec = self.dec16(bc);
                self.set_bc(dec);
                self.cycles += 8;
                pc += 1;
            },
            0x0c => {
                trace!("inc c");
                let c = self.c;
                self.c = self.inc(c);
                self.cycles += 4;
                pc += 1;
            },
            0x0d => {
                trace!("dec c");
                let c = self.c;
                self.c = self.dec(c);
                self.cycles += 4;
                pc += 1;
            },
            0x0e => {
                let val = mm.rom[pc + 1];
                trace!("ld c, ${:02x}", val);
                self.c = val;
                let c = self.c;
                self.set_zero(c == 0);
                self.cycles += 8;
                pc += 2;
            },
            0x0f => {
                trace!("rrca");
                let a = self.a;
                self.a = self.rrc(a);
                self.cycles += 4;
                pc += 1;
            },
            0x10 => {
                trace!("stop");
                // TODO
                self.cycles += 4;
                pc += 2;
            },
            0x11 => {
                let val = self.read_u16(mm, pc + 1);
                trace!("ld de, ${:04x}", val);
                self.set_de(val);
                self.cycles += 12;
                pc += 3;
            },
            0x12 => {
                trace!("ld (de), a");
                mm.write(self.de(), self.a);
                self.cycles += 8;
                pc += 1;
            },
            0x13 => {
                trace!("inc de");
                let de = self.de();
                let inc = self.inc16(de);
                self.set_de(inc);
                self.cycles += 8;
                pc += 1;
            },
            0x14 => {
                trace!("inc d");
                let d = self.d;
                self.d = self.inc(d);
                self.cycles += 4;
                pc += 1;
            },
            0x15 => {
                trace!("dec d");
                let d = self.d;
                self.d = self.dec(d);
                self.cycles += 4;
                pc += 1;
            },
            0x16 => {
                let val = mm.rom[pc + 1];
                trace!("ld d, ${:02x}", val);
                self.d = val;
                self.cycles += 8;
                pc += 2;
            },
            0x17 => {
                trace!("rla");
                let a = self.a;
                self.a = self.rl(a);
                self.cycles += 4;
                pc += 1;
            },
            0x18 => {
                let val = mm.rom[pc + 1] as i8;
                trace!("jr ${:02x}", val);
                pc = ((pc as isize) + (val as isize)) as usize;
                self.cycles += 12;
                pc += 2;
            },
            0x19 => {
                trace!("add hl, de");
                let de = self.de();
                let hl = self.hl();
                self.set_hl(hl.wrapping_add(de));
                self.set_subtract(false);
                self.set_half_carry(false);
                self.set_carry(false /* TODO */);
                self.cycles += 8;
                pc += 1;
            },
            0x1a => {
                trace!("ld a, (de)");
                self.a = mm.read(self.de());
                self.cycles += 8;
                pc += 1;
            },
            0x1b => {
                trace!("dec de");
                let de = self.de();
                let dec = self.dec16(de);
                self.set_de(dec);
                self.cycles += 8;
                pc += 1;
            },
            0x1c => {
                trace!("inc e");
                let e = self.e;
                self.e = self.inc(e);
                self.cycles += 4;
                pc += 1;
            },
            0x1d => {
                trace!("dec e");
                let e = self.e;
                self.e = self.dec(e);
                self.cycles += 4;
                pc += 1;
            },
            0x1e => {
                let val = mm.rom[pc + 1];
                trace!("ld e, ${:02x}", val);
                self.e = val;
                self.cycles += 8;
                pc += 2;
            },
            0x1f => {
                trace!("rra");
                let a = self.a;
                self.a = self.rr(a);
                self.cycles += 4;
                pc += 1;
            },
            0x20 => {
                let val = mm.rom[pc + 1] as i8;
                trace!("jr nz, #{}", val);
                if !self.zero() {
                    pc = ((pc as isize) + (val as isize)) as usize;
                    self.cycles += 12;
                } else {
                    self.cycles += 8;
                }
                pc += 2;
            },
            0x21 => {
                let val = self.read_u16(mm, pc + 1);
                trace!("ld hl, ${:04x}", val);
                self.set_hl(val);
                self.cycles += 12;
                pc += 3;
            },
            0x22 => {
                trace!("ld (hl+), a");
                let hl = self.hl();
                mm.write(hl, self.a);
                self.set_hl(hl.wrapping_add(1));
                self.cycles += 8;
                pc += 1;
            },
            0x23 => {
                trace!("inc hl");
                let hl = self.hl();
                let inc = self.inc16(hl);
                self.set_hl(inc);
                self.cycles += 8;
                pc += 1;
            },
            0x24 => {
                trace!("inc h");
                let h = self.h;
                self.h = self.inc(h);
                self.cycles += 4;
                pc += 1;
            },
            0x25 => {
                trace!("dec h");
                let h = self.h;
                self.h = self.dec(h);
                self.cycles += 4;
                pc += 1;
            },
            0x26 => {
                let val = mm.rom[pc + 1];
                trace!("ld h, ${:02x}", val);
                self.h = val;
                self.cycles += 8;
                pc += 2;
            },
            0x27 => {
                trace!("daa");
                // TODO
                self.cycles += 4;
                pc += 1;
            },
            0x28 => {
                let val = mm.rom[pc + 1] as i8;
                trace!("jr z, #{}", val);
                if self.zero() {
                    pc = ((pc as isize) + (val as isize)) as usize;
                    self.cycles += 12;
                } else {
                    self.cycles += 8;
                }
                pc += 2;
            },
            0x29 => {
                trace!("add hl, hl");
                let hl = self.hl();
                self.set_hl(hl.wrapping_add(hl));
                self.cycles += 8;
                pc += 1;
            },
            0x2a => {
                trace!("ld a, (hl+)");
                let hl = self.hl();
                self.a = mm.read(hl);
                let inc = self.inc16(hl);
                self.set_hl(inc);
                self.cycles += 8;
                pc += 1;
            },
            0x2b => {
                trace!("dec hl");
                let hl = self.hl();
                let dec = self.dec16(hl);
                self.set_hl(dec);
                self.cycles += 8;
                pc += 1;
            },
            0x2c => {
                trace!("inc l");
                let l = self.l;
                self.l = self.inc(l);
                self.cycles += 4;
                pc += 1;
            },
            0x2d => {
                trace!("dec l");
                let l = self.l;
                self.l = self.dec(l);
                self.cycles += 4;
                pc += 1;
            },
            0x2e => {
                let val = mm.rom[pc + 1];
                trace!("ld l, ${:02x}", val);
                self.l = val;
                self.cycles += 8;
                pc += 2;
            },
            0x2f => {
                trace!("cpl");
                self.a = !self.a;
                let a = self.a;
                self.set_zero(a == 0);
                self.cycles += 4;
                pc += 1;
            },
            0x30 => {
                let val = mm.rom[pc + 1] as i8;
                trace!("jr nc, #{}", val);
                if !self.carry() {
                    pc = ((pc as isize) + (val as isize)) as usize;
                    self.cycles += 12;
                } else {
                    self.cycles += 8;
                }
                pc += 2;
            },
            0x31 => {
                let val = self.read_u16(mm, pc + 1);
                trace!("ld sp, ${:04x}", val);
                self.sp = val;
                self.cycles += 12;
                pc += 3;
            },
            0x32 => {
                trace!("ld (hl-), a");
                let hl = self.hl();
                mm.write(hl, self.a);
                let dec = self.dec16(hl);
                self.set_hl(dec);
                self.cycles += 8;
                pc += 1;
            },
            0x33 => {
                trace!("inc sp");
                let sp = self.sp;
                self.sp = self.inc16(sp);
                self.cycles += 8;
                pc += 1;
            },
            0x34 => {
                trace!("inc (hl)");
                let hl = self.hl();
                let val = mm.read(hl);
                mm.write(hl, val.wrapping_add(1));
                self.cycles += 12;
                pc += 1;
            },
            0x35 => {
                trace!("dec (hl)");
                let hl = self.hl();
                let val = mm.read(hl);
                mm.write(hl, val.wrapping_sub(1));
                self.cycles += 12;
                pc += 1;
            },
            0x36 => {
                let val = mm.rom[pc + 1];
                trace!("ld (hl), ${:02x}", val);
                mm.write(self.hl(), val);
                self.cycles += 12;
                pc += 1;
            },
            0x37 => {
                panic!("scf");
                self.cycles += 4;
                pc += 1;
            },
            0x38 => {
                let val = mm.rom[pc + 1] as i8;
                trace!("jr c, #{}", val);
                if self.carry() {
                    pc = ((pc as isize) + (val as isize)) as usize;
                    self.cycles += 12;
                } else {
                    self.cycles += 8;
                }
                pc += 2;
            },
            0x39 => {
                trace!("add hl, sp");
                let hl = self.hl();
                let sp = self.sp;
                self.set_hl(hl.wrapping_add(sp));
                self.cycles += 8;
                pc += 2;
            },
            0x3a => {
                trace!("ld a, (hl-)");
                self.a = mm.read(self.hl());
                let hl = self.hl();
                self.set_hl(hl.wrapping_sub(1));
                self.cycles += 8;
                pc += 2;
            },
            0x3b => {
                trace!("dec sp");
                let sp = self.sp;
                self.sp = self.dec16(sp);
                self.cycles += 8;
                pc += 2;
            },
            0x3c => {
                trace!("inc a");
                let a = self.a;
                self.a = self.inc(a);
                self.cycles += 4;
                pc += 1;
            },
            0x3d => {
                trace!("dec a");
                let a = self.a;
                self.a = self.dec(a);
                self.cycles += 4;
                pc += 1;
            },
            0x3e => {
                let val = mm.rom[pc + 1];
                trace!("ld a, ${:02x}", val);
                self.a = val;
                self.cycles += 8;
                pc += 2;
            },
            0x3f => {
                panic!("ccf");
                self.cycles += 4;
                pc += 1;
            },
            0x40 => {
                trace!("ld b, b");
                self.b = self.b;
                self.cycles += 4;
                pc += 1;
            },
            0x41 => {
                trace!("ld b, c");
                self.b = self.c;
                self.cycles += 4;
                pc += 1;
            },
            0x42 => {
                trace!("ld b, d");
                self.b = self.d;
                self.cycles += 4;
                pc += 1;
            },
            0x43 => {
                trace!("ld b, e");
                self.b = self.e;
                self.cycles += 4;
                pc += 1;
            },
            0x44 => {
                trace!("ld b, h");
                self.b = self.h;
                self.cycles += 4;
                pc += 1;
            },
            0x45 => {
                trace!("ld b, l");
                self.b = self.l;
                self.cycles += 4;
                pc += 1;
            },
            0x46 => {
                trace!("ld b, (hl)");
                self.b = mm.read(self.hl());
                self.cycles += 8;
                pc += 1;
            },
            0x47 => {
                trace!("ld b, a");
                self.b = self.a;
                self.cycles += 4;
                pc += 1;
            },
            0x48 => {
                trace!("ld c, b");
                self.c = self.b;
                self.cycles += 4;
                pc += 1;
            },
            0x49 => {
                trace!("ld c, c");
                self.c = self.c;
                self.cycles += 4;
                pc += 1;
            },
            0x4a => {
                trace!("ld c, d");
                self.c = self.d;
                self.cycles += 4;
                pc += 1;
            },
            0x4b => {
                trace!("ld c, e");
                self.c = self.e;
                self.cycles += 4;
                pc += 1;
            },
            0x4c => {
                trace!("ld c, h");
                self.c = self.h;
                self.cycles += 4;
                pc += 1;
            },
            0x4d => {
                trace!("ld c, l");
                self.c = self.l;
                self.cycles += 4;
                pc += 1;
            },
            0x4e => {
                trace!("ld c, (hl)");
                self.c = mm.read(self.hl());
                self.cycles += 8;
                pc += 1;
            },
            0x4f => {
                trace!("ld c, a");
                self.c = self.a;
                self.cycles += 4;
                pc += 1;
            },
            0x50 => {
                trace!("ld c, b");
                self.c = self.b;
                self.cycles += 4;
                pc += 1;
            },
            0x51 => {
                trace!("ld c, c");
                self.c = self.c;
                self.cycles += 4;
                pc += 1;
            },
            0x52 => {
                trace!("ld c, d");
                self.c = self.d;
                self.cycles += 4;
                pc += 1;
            },
            0x53 => {
                trace!("ld c, e");
                self.c = self.e;
                self.cycles += 4;
                pc += 1;
            },
            0x54 => {
                trace!("ld c, h");
                self.c = self.h;
                self.cycles += 4;
                pc += 1;
            },
            0x55 => {
                trace!("ld c, l");
                self.c = self.l;
                self.cycles += 4;
                pc += 1;
            },
            0x56 => {
                trace!("ld c, (hl)");
                self.c = mm.read(self.hl());
                self.cycles += 8;
                pc += 1;
            },
            0x57 => {
                trace!("ld c, a");
                self.c = self.a;
                self.cycles += 4;
                pc += 1;
            },
            0x58 => {
                trace!("ld e, b");
                self.e = self.b;
                self.cycles += 4;
                pc += 1;
            },
            0x59 => {
                trace!("ld e, c");
                self.e = self.c;
                self.cycles += 4;
                pc += 1;
            },
            0x5a => {
                trace!("ld e, d");
                self.e = self.d;
                self.cycles += 4;
                pc += 1;
            },
            0x5b => {
                trace!("ld e, e");
                self.e = self.e;
                self.cycles += 4;
                pc += 1;
            },
            0x5c => {
                trace!("ld e, h");
                self.e = self.h;
                self.cycles += 4;
                pc += 1;
            },
            0x5d => {
                trace!("ld e, l");
                self.e = self.l;
                self.cycles += 4;
                pc += 1;
            },
            0x5e => {
                trace!("ld e, (hl)");
                self.e = mm.read(self.hl());
                self.cycles += 8;
                pc += 1;
            },
            0x5f => {
                trace!("ld e, a");
                self.e = self.a;
                self.cycles += 4;
                pc += 1;
            },
            0x60 => {
                trace!("ld h, b");
                self.h = self.b;
                self.cycles += 4;
                pc += 1;
            },
            0x61 => {
                trace!("ld h, c");
                self.h = self.c;
                self.cycles += 4;
                pc += 1;
            },
            0x62 => {
                trace!("ld h, d");
                self.h = self.d;
                self.cycles += 4;
                pc += 1;
            },
            0x63 => {
                trace!("ld h, e");
                self.h = self.e;
                self.cycles += 4;
                pc += 1;
            },
            0x64 => {
                trace!("ld h, h");
                self.h = self.h;
                self.cycles += 4;
                pc += 1;
            },
            0x65 => {
                trace!("ld h, l");
                self.h = self.l;
                self.cycles += 4;
                pc += 1;
            },
            0x66 => {
                trace!("ld h, (hl)");
                self.h = mm.read(self.hl());
                self.cycles += 8;
                pc += 1;
            },
            0x67 => {
                trace!("ld h, a");
                self.h = self.a;
                self.cycles += 4;
                pc += 1;
            },
            0x68 => {
                trace!("ld l, b");
                self.l = self.b;
                self.cycles += 4;
                pc += 1;
            },
            0x69 => {
                trace!("ld l, c");
                self.l = self.c;
                self.cycles += 4;
                pc += 1;
            },
            0x6a => {
                trace!("ld l, d");
                self.l = self.d;
                self.cycles += 4;
                pc += 1;
            },
            0x6b => {
                trace!("ld l, e");
                self.l = self.e;
                self.cycles += 4;
                pc += 1;
            },
            0x6c => {
                trace!("ld l, h");
                self.l = self.h;
                self.cycles += 4;
                pc += 1;
            },
            0x6d => {
                trace!("ld l, l");
                self.l = self.l;
                self.cycles += 4;
                pc += 1;
            },
            0x6e => {
                trace!("ld l, (hl)");
                self.l = mm.read(self.hl());
                self.cycles += 8;
                pc += 1;
            },
            0x6f => {
                trace!("ld l, a");
                self.l = self.a;
                self.cycles += 4;
                pc += 1;
            },
            0x70 => {
                trace!("ld (hl), b");
                mm.write(self.hl(), self.b);
                self.cycles += 8;
                pc += 1;
            },
            0x71 => {
                trace!("ld (hl), c");
                mm.write(self.hl(), self.c);
                self.cycles += 8;
                pc += 1;
            },
            0x72 => {
                trace!("ld (hl), d");
                mm.write(self.hl(), self.d);
                self.cycles += 8;
                pc += 1;
            },
            0x73 => {
                trace!("ld (hl), e");
                mm.write(self.hl(), self.e);
                self.cycles += 8;
                pc += 1;
            },
            0x74 => {
                trace!("ld (hl), h");
                mm.write(self.hl(), self.h);
                self.cycles += 8;
                pc += 1;
            },
            0x75 => {
                trace!("ld (hl), l");
                mm.write(self.hl(), self.l);
                self.cycles += 8;
                pc += 1;
            },
            0x76 => {
                panic!("halt");
            },
            0x77 => {
                trace!("ld (hl), a");
                mm.write(self.hl(), self.a);
                self.cycles += 8;
                pc += 1;
            },
            0x78 => {
                trace!("ld a, b");
                self.a = self.b;
                self.cycles += 4;
                pc += 1;
            },
            0x79 => {
                trace!("ld a, c");
                self.a = self.c;
                self.cycles += 4;
                pc += 1;
            },
            0x7a => {
                trace!("ld a, d");
                self.a = self.d;
                self.cycles += 4;
                pc += 1;
            },
            0x7b => {
                trace!("ld a, e");
                self.a = self.e;
                self.cycles += 4;
                pc += 1;
            },
            0x7c => {
                trace!("ld a, h");
                self.a = self.h;
                self.cycles += 4;
                pc += 1;
            },
            0x7d => {
                trace!("ld a, l");
                self.a = self.l;
                self.cycles += 4;
                pc += 1;
            },
            0x7e => {
                trace!("ld a, (hl)");
                self.a = mm.read(self.hl());
                self.cycles += 8;
                pc += 1;
            },
            0x7f => {
                trace!("ld a, a");
                self.a = self.a;
                self.cycles += 4;
                pc += 1;
            },
            0x80 => {
                trace!("add b");
                let val = self.b;
                self.add(val);
                self.cycles += 4;
                pc += 1;
            },
            0x81 => {
                trace!("add c");
                let val = self.c;
                self.add(val);
                self.cycles += 4;
                pc += 1;
            },
            0x82 => {
                trace!("add d");
                let val = self.d;
                self.add(val);
                self.cycles += 4;
                pc += 1;
            },
            0x83 => {
                trace!("add e");
                let val = self.e;
                self.add(val);
                self.cycles += 4;
                pc += 1;
            },
            0x84 => {
                trace!("add h");
                let val = self.h;
                self.add(val);
                self.cycles += 4;
                pc += 1;
            },
            0x85 => {
                trace!("add l");
                let val = self.l;
                self.add(val);
                self.cycles += 4;
                pc += 1;
            },
            0x86 => {
                trace!("add (hl)");
                let val = mm.read(self.hl());
                self.add(val);
                self.cycles += 8;
                pc += 1;
            },
            0x87 => {
                trace!("add a");
                let val = self.a;
                self.add(val);
                self.cycles += 8;
                pc += 1;
            },
            0x88 => {
                trace!("adc b");
                let val = self.b;
                self.adc(val);
                self.cycles += 4;
                pc += 1;
            },
            0x89 => {
                trace!("adc c");
                let val = self.c;
                self.adc(val);
                self.cycles += 4;
                pc += 1;
            },
            0x8a => {
                trace!("adc d");
                let val = self.d;
                self.adc(val);
                self.cycles += 4;
                pc += 1;
            },
            0x8b => {
                trace!("adc e");
                let val = self.e;
                self.adc(val);
                self.cycles += 4;
                pc += 1;
            },
            0x8c => {
                trace!("adc h");
                let val = self.h;
                self.adc(val);
                self.cycles += 4;
                pc += 1;
            },
            0x8d => {
                trace!("adc l");
                let val = self.l;
                self.adc(val);
                self.cycles += 4;
                pc += 1;
            },
            0x8e => {
                trace!("adc (hl)");
                let val = mm.read(self.hl());;
                self.adc(val);
                self.cycles += 8;
                pc += 1;
            },
            0x8f => {
                trace!("adc a");
                let val = self.a;
                self.adc(val);
                self.cycles += 4;
                pc += 1;
            },
            0x90 => {
                trace!("sub b");
                let val = self.b;
                self.sub(val);
                self.cycles += 4;
                pc += 1;
            },
            0x91 => {
                trace!("sub c");
                let val = self.c;
                self.sub(val);
                self.cycles += 4;
                pc += 1;
            },
            0x92 => {
                trace!("sub d");
                let val = self.d;
                self.sub(val);
                self.cycles += 4;
                pc += 1;
            },
            0x93 => {
                trace!("sub e");
                let val = self.e;
                self.sub(val);
                self.cycles += 4;
                pc += 1;
            },
            0x94 => {
                trace!("sub h");
                let val = self.h;
                self.sub(val);
                self.cycles += 4;
                pc += 1;
            },
            0x95 => {
                trace!("sub l");
                let val = self.l;
                self.sub(val);
                self.cycles += 4;
                pc += 1;
            },
            0x96 => {
                trace!("sub (hl)");
                let val = mm.read(self.hl());
                self.sub(val);
                self.cycles += 8;
                pc += 1;
            },
            0x97 => {
                trace!("sub a");
                let val = self.a;
                self.sub(val);
                self.cycles += 4;
                pc += 1;
            },
            0x98 => {
                trace!("sbc b");
                let val = self.b;
                self.sbc(val);
                self.cycles += 4;
                pc += 1;
            },
            0x99 => {
                trace!("sbc c");
                let val = self.c;
                self.sbc(val);
                self.cycles += 4;
                pc += 1;
            },
            0x9a => {
                trace!("sbc d");
                let val = self.d;
                self.sbc(val);
                self.cycles += 4;
                pc += 1;
            },
            0x9b => {
                trace!("sbc e");
                let val = self.e;
                self.sbc(val);
                self.cycles += 4;
                pc += 1;
            },
            0x9c => {
                trace!("sbc h");
                let val = self.h;
                self.sbc(val);
                self.cycles += 4;
                pc += 1;
            },
            0x9d => {
                trace!("sbc l");
                let val = self.l;
                self.sbc(val);
                self.cycles += 4;
                pc += 1;
            },
            0x9e => {
                trace!("sbc (hl)");
                let val = mm.read(self.hl());
                self.sbc(val);
                self.cycles += 8;
                pc += 1;
            },
            0x9f => {
                trace!("sbc a");
                let val = self.a;
                self.sbc(val);
                self.cycles += 4;
                pc += 1;
            },
            0xa0 => {
                trace!("and b");
                let val = self.a;
                self.and(val);
                self.cycles += 4;
                pc += 1;
            },
            0xa1 => {
                trace!("and c");
                let val = self.c;
                self.and(val);
                self.cycles += 4;
                pc += 1;
            },
            0xa2 => {
                trace!("and d");
                let val = self.d;
                self.and(val);
                self.cycles += 4;
                pc += 1;
            },
            0xa3 => {
                trace!("and e");
                let val = self.e;
                self.and(val);
                self.cycles += 4;
                pc += 1;
            },
            0xa4 => {
                trace!("and h");
                let val = self.h;
                self.and(val);
                self.cycles += 4;
                pc += 1;
            },
            0xa5 => {
                trace!("and l");
                let val = self.l;
                self.and(val);
                self.cycles += 4;
                pc += 1;
            },
            0xa6 => {
                trace!("and (hl)");
                let val = mm.read(self.hl());
                self.and(val);
                self.cycles += 8;
                pc += 1;
            },
            0xa7 => {
                trace!("and a");
                let val = self.a;
                self.and(val);
                self.cycles += 4;
                pc += 1;
            },
            0xa8 => {
                trace!("xor b");
                let val = self.b;
                self.xor(val);
                self.cycles += 4;
                pc += 1;
            },
            0xa9 => {
                trace!("xor c");
                let val = self.c;
                self.xor(val);
                self.cycles += 4;
                pc += 1;
            },
            0xaa => {
                trace!("xor d");
                let val = self.d;
                self.xor(val);
                self.cycles += 4;
                pc += 1;
            },
            0xab => {
                trace!("xor e");
                let val = self.e;
                self.xor(val);
                self.cycles += 4;
                pc += 1;
            },
            0xac => {
                trace!("xor h");
                let val = self.h;
                self.xor(val);
                self.cycles += 4;
                pc += 1;
            },
            0xad => {
                trace!("xor l");
                let val = self.l;
                self.xor(val);
                self.cycles += 4;
                pc += 1;
            },
            0xae => {
                trace!("xor (hl)");
                let val = mm.read(self.hl());
                self.xor(val);
                self.cycles += 8;
                pc += 1;
            },
            0xaf => {
                trace!("xor a");
                let val = self.a;
                self.xor(val);
                self.cycles += 4;
                pc += 1;
            },
            0xb0 => {
                trace!("or b");
                let val = self.b;
                self.or(val);
                self.cycles += 4;
                pc += 1;
            },
            0xb1 => {
                trace!("or c");
                let val = self.c;
                self.or(val);
                self.cycles += 4;
                pc += 1;
            },
            0xb2 => {
                trace!("or d");
                let val = self.d;
                self.or(val);
                self.cycles += 4;
                pc += 1;
            },
            0xb3 => {
                trace!("or e");
                let val = self.e;
                self.or(val);
                self.cycles += 4;
                pc += 1;
            },
            0xb4 => {
                trace!("or h");
                let val = self.h;
                self.or(val);
                self.cycles += 4;
                pc += 1;
            },
            0xb5 => {
                trace!("or l");
                let val = self.l;
                self.or(val);
                self.cycles += 4;
                pc += 1;
            },
            0xb6 => {
                trace!("or (hl)");
                let val = mm.read(self.hl());
                self.or(val);
                self.cycles += 8;
                pc += 1;
            },
            0xb7 => {
                trace!("or a");
                let val = self.a;
                self.or(val);
                self.cycles += 4;
                pc += 1;
            },
            0xb8 => {
                trace!("cp b");
                let val = self.b;
                self.cp(val);
                self.cycles += 4;
                pc += 1;
            },
            0xb9 => {
                trace!("cp c");
                let val = self.c;
                self.cp(val);
                self.cycles += 4;
                pc += 1;
            },
            0xba => {
                trace!("cp d");
                let val = self.d;
                self.cp(val);
                self.cycles += 4;
                pc += 1;
            },
            0xbb => {
                trace!("cp e");
                let val = self.e;
                self.cp(val);
                self.cycles += 4;
                pc += 1;
            },
            0xbc => {
                trace!("cp h");
                let val = self.h;
                self.cp(val);
                self.cycles += 4;
                pc += 1;
            },
            0xbd => {
                trace!("cp l");
                let val = self.l;
                self.cp(val);
                self.cycles += 4;
                pc += 1;
            },
            0xbe => {
                trace!("cp (hl)");
                let val = mm.read(self.hl());
                self.cp(val);
                self.cycles += 8;
                pc += 1;
            },
            0xbf => {
                trace!("cp a");
                let val = self.a;
                self.cp(val);
                self.cycles += 4;
                pc += 1;
            },
            0xc0 => {
                trace!("ret nz");
                if !self.zero() {
                    let addr = self.read_return_addr(mm);
                    self.cycles += 20;
                    pc = addr as usize;
                } else {
                    self.cycles += 8;
                    pc += 1;
                }
            },
            0xc1 => {
                panic!("pop bc");
                self.cycles += 12;
                pc += 1;
            },
            0xc2 => {
                let val = self.read_u16(mm, pc + 1);
                trace!("jp nz, ${:04x}", val);
                if !self.zero() {
                    self.cycles += 16;
                    pc = val as usize;
                } else {
                    self.cycles += 12;
                    pc += 3;
                }
            },
            0xc3 => {
                let val = self.read_u16(mm, pc + 1);
                trace!("jp ${:04x}", val);
                self.cycles += 16;
                pc = val as usize;
            },
            0xc4 => {
                let val = self.read_u16(mm, pc + 1);
                trace!("call nz, ${:04x}", val);
                if !self.zero() {
                    let addr = self.pc + 3;
                    self.write_return_addr(mm, addr);
                    self.cycles += 24;
                    pc = val as usize;
                } else {
                    self.cycles += 12;
                    pc += 3;
                }
            },
            0xc5 => {
                panic!("push bc");
                self.cycles += 12; // 24
                pc += 1;
            },
            0xc6 => {
                let val = mm.rom[pc + 1];
                trace!("add a, ${:02x}", val);
                self.add(val);
                self.cycles += 8;
                pc += 2;
            },
            0xc7 => {
                panic!("rst 00");
                self.cycles += 16;
                pc += 1;
            },
            0xc8 => {
                trace!("ret z");
                if self.zero() {
                    let addr = self.read_return_addr(mm);
                    self.cycles += 20;
                    pc = addr as usize;
                } else {
                    self.cycles += 8;
                    pc += 1;
                }
            },
            0xc9 => {
                trace!("ret");
                let addr = self.read_return_addr(mm);
                self.cycles += 16;
                pc = addr as usize;
            },
            0xca => {
                let val = self.read_u16(mm, pc + 1);
                trace!("jp z, ${:04x}", val);
                if self.zero() {
                    self.cycles += 16;
                    pc = val as usize;
                } else {
                    self.cycles += 12;
                    pc += 3;
                }
            },
            0xcb => {
                panic!("prefix cb");
                self.cycles += 4;
                pc += 1;
            },
            0xcc => {
                let val = self.read_u16(mm, pc + 1);
                trace!("call z, ${:04x}", val);
                if self.zero() {
                    let addr = self.pc + 3;
                    self.write_return_addr(mm, addr);
                    self.cycles += 24;
                    pc = val as usize;
                } else {
                    self.cycles += 12;
                    pc += 3;
                }
            },
            0xcd => {
                let val = self.read_u16(mm, pc + 1);
                trace!("call ${:04x}", val);
                let addr = self.pc + 3;
                self.write_return_addr(mm, addr);
                self.cycles += 24;
                pc = val as usize;
            },
            0xce => {
                let val = mm.rom[pc + 1];
                trace!("adc ${:02x}", val);
                self.adc(val);
                self.cycles += 8;
                pc += 2;
            },
            0xcf => {
                panic!("rst 08");
                self.cycles += 16;
                pc += 1;
            },
            0xd0 => {
                trace!("ret nc");
                if !self.carry() {
                    let addr = self.read_return_addr(mm);
                    self.cycles += 20;
                    pc = addr as usize;
                } else {
                    self.cycles += 8;
                    pc += 1;
                }
            },
            0xd1 => {
                panic!("pop de");
                self.cycles += 12;
                pc += 1;
            },
            0xd2 => {
                let val = self.read_u16(mm, pc + 1);
                trace!("jp nc, ${:04x}", val);
                if !self.carry() {
                    self.cycles += 16;
                    pc = val as usize;
                } else {
                    self.cycles += 12;
                    pc += 3;
                }
            },
            0xd4 => {
                let val = self.read_u16(mm, pc + 1);
                trace!("call nc, ${:04x}", val);
                if !self.carry() {
                    let addr = self.pc + 3;
                    self.write_return_addr(mm, addr);
                    self.cycles += 24;
                    pc = val as usize;
                } else {
                    self.cycles += 12;
                    pc += 3;
                }
            },
            0xd5 => {
                panic!("push de");
                self.cycles += 16;
                pc += 1;
            },
            0xd6 => {
                let val = mm.rom[pc + 1];
                trace!("sub ${:02x}", val);
                self.sub(val);
                self.cycles += 8;
                pc += 2;
            },
            0xd7 => {
                panic!("rst 10");
                self.cycles += 16;
                pc += 1;
            },
            0xd8 => {
                trace!("ret c");
                if self.carry() {
                    let addr = self.read_return_addr(mm);
                    self.cycles += 20;
                    pc = addr as usize;
                } else {
                    self.cycles += 8;
                    pc += 1;
                }
            },
            0xd9 => {
                panic!("reti");
                self.cycles += 16;
                pc += 1;
            },
            0xda => {
                let val = self.read_u16(mm, pc + 1);
                trace!("jp c, ${:04x}", val);
                if self.carry() {
                    self.cycles += 16;
                    pc = val as usize;
                } else {
                    self.cycles += 12;
                    pc += 3;
                }
            },
            0xdc => {
                let val = self.read_u16(mm, pc + 1);
                trace!("call c, ${:04x}", val);
                if self.carry() {
                    let addr = self.pc + 3;
                    self.write_return_addr(mm, addr);
                    self.cycles += 24;
                    pc = val as usize;
                } else {
                    self.cycles += 12;
                    pc += 3;
                }
            },
            0xde => {
                let val = mm.rom[pc + 1];
                trace!("sbc ${:02x}", val);
                self.sbc(val);
                self.cycles += 8;
                pc += 2;
            },
            0xdf => {
                panic!("rst 18");
                self.cycles += 16;
                pc += 1;
            },
            0xe0 => {
                let val = mm.rom[pc + 1];
                trace!("ld ($ff00+{:02x}), a", val);
                let addr = 0xff00 + val as u16;
                trace!("a = {}", self.a);
                mm.write(addr, self.a);
                self.cycles += 12;
                pc += 2;
            },
            0xe1 => {
                panic!("pop hl");
                self.cycles += 12;
                pc += 1;
            },
            0xe2 => {
                trace!("ld ($ff00+c), a");
                let addr = 0xff00 + self.c as u16;
                mm.write(addr, self.a);
                self.cycles += 8;
                pc += 1;
            },
            0xe5 => {
                panic!("push hl");
                self.cycles += 16;
                pc += 1;
            },
            0xe6 => {
                let val = mm.rom[pc + 1];
                trace!("and ${:02x}", val);
                self.and(val);
                self.cycles += 8;
                pc += 2;
            },
            0xe7 => {
                panic!("rst $20");
                self.cycles += 16;
                pc += 1;
            },
            0xe8 => {
                let val = mm.rom[pc + 1];
                trace!("add sp, ${:02x}", val);
                self.sp += val as u16;
                self.cycles += 16;
                pc += 2;
            },
            0xe9 => {
                panic!("jp (hl)");
                self.cycles += 4;
                pc += 1;
            },
            0xea => {
                let val = self.read_u16(mm, pc + 1);
                trace!("ld (${:04x}), a", val);
                self.cycles += 16;
                pc += 3;
            },
            0xee => {
                let val = mm.rom[pc + 1];
                trace!("xor ${:02x}", val);
                self.xor(val);
                self.cycles += 8;
                pc += 2;
            },
            0xef => {
                panic!("rst $28");
                self.cycles += 16;
                pc += 1;
            },
            0xf0 => {
                let val = mm.rom[pc + 1];
                trace!("ld a, ($ff00+{:02x})", val);
                let addr = 0xff00 + val as u16;
                self.a = mm.read(addr);
                self.cycles += 12;
                pc += 2;
            },
            0xf1 => {
                panic!("pop af");
                self.cycles += 12;
                pc += 1;
            },
            0xf2 => {
                trace!("ld a, ($ff00+c)");
                let addr = 0xff00 + self.c as u16;
                self.a = mm.read(addr);
                self.cycles += 8;
                pc += 1;
            },
            0xf3 => {
                trace!("di");
                mm.di();
                self.cycles += 4;
                pc += 1;
            },
            0xf5 => {
                panic!("push af");
                self.cycles += 16;
                pc += 1;
            },
            0xf6 => {
                let val = mm.rom[pc + 1];
                trace!("or ${:02x}", val);
                self.or(val);
                self.cycles += 8;
                pc += 2;
            },
            0xf7 => {
                panic!("rst $30");
                self.cycles += 16;
                pc += 1;
            },
            0xf8 => {
                let val = mm.rom[pc + 1];
                panic!("ld hl, sp+${:02x}", val);
                self.cycles += 12;
                pc += 2;
            },
            0xf9 => {
                panic!("ld sp, hl");
                self.cycles += 8;
                pc += 1;
            },
            0xfa => {
                let val = self.read_u16(mm, pc + 1);
                panic!("ld a, (${:04x})", val);
                self.cycles += 16;
                pc += 3;
            },
            0xfb => {
                trace!("ei");
                mm.ei();
                self.cycles += 4;
                pc += 1;
            },
            0xfe => {
                let val = mm.rom[pc + 1];
                trace!("cp ${:02x}", val);
                self.cp(val);
                self.cycles += 8;
                pc += 2;
            },
            0xff => {
                panic!("rst $38");
                self.cycles += 16;
                pc += 1;
            },
            _ => panic!("unknown instruction {:02x} @ pc={:04x}", mm.rom[pc], pc),
        }
        self.pc = pc as u16;
        return self.cycles;
    }
}

#[test]
fn test_cpu() {
    let mut cpu = Cpu::new();

    cpu.set_af(0x2343);
    assert_eq!(cpu.a, 0x23);
    assert_eq!(cpu.f, 0x43);
    assert_eq!(cpu.af(), 0x2343);
    cpu.set_bc(0x5432);
    assert_eq!(cpu.b, 0x54);
    assert_eq!(cpu.c, 0x32);
    assert_eq!(cpu.bc(), 0x5432);
    cpu.set_de(0x9988);
    assert_eq!(cpu.d, 0x99);
    assert_eq!(cpu.e, 0x88);
    assert_eq!(cpu.de(), 0x9988);
    cpu.set_hl(0x8743);
    assert_eq!(cpu.h, 0x87);
    assert_eq!(cpu.l, 0x43);
    assert_eq!(cpu.hl(), 0x8743);

    cpu.f = 0;
    cpu.set_zero(true);
    assert_eq!(cpu.f, 0x80);
    cpu.set_zero(false);
    assert_eq!(cpu.f, 0);

    let rom = vec![0x00, 0x01, 0x23, 0x45];
    let vram : [u8; 0x2000] = [0; 0x2000];
    let wram : [u8; 0x2000] = [0; 0x2000];
    let hram : [u8; 0x80] = [0; 0x80];
    let iobuf : [u8; 0x100] = [0; 0x100];
    let lcd = Rc::new(RefCell::new(lcd::Lcd::new()));
    let timer = Rc::new(RefCell::new(timer::Timer::new()));
    let joypad = Rc::new(RefCell::new(joypad::Joypad::new()));
    let mut mm = mem::MemoryMap { rom: rom, vram: vram, wram: wram, hram: hram,
    iobuf: iobuf, interrupt_enable: 0, interrupt_master_enable: false,
    oam: [0; 0xa0],
    interrupt_flag: 0,
    lcd: lcd,
    timer: timer,
    joypad: joypad,
    };
    assert_eq!(cpu.read_u16(&mut mm, 0), 0x0100);
    assert_eq!(cpu.read_u16(&mut mm, 2), 0x4523);

    cpu.write_return_addr(&mut mm, 0x1234);
    assert_eq!(cpu.sp, 0xfffc);
    assert_eq!(mm.read(cpu.sp), 0x34);
    assert_eq!(mm.read(cpu.sp + 1), 0x12);
    assert_eq!(cpu.read_return_addr(&mut mm), 0x1234);

    assert_eq!(cpu.rlc(23), 2*23);
    assert_eq!(cpu.rlc(46), 2*46);
    assert_eq!(cpu.rlc(92), 2*92);
    assert_eq!(cpu.rlc(150), 45);

    assert_eq!(cpu.rrc(150), 75);
    assert_eq!(cpu.rrc(75), 165);

    {
        let a : num::Wrapping<u8>;
        let b : num::Wrapping<u8>;

        a = num::Wrapping(23);
        b = num::Wrapping(42);

        //a += num::Wrapping(4);

        let c = a + b;
        println!("a = {}", a.0);
        println!("b = {}", b.0);
        println!("c = {}", c.0);
    }

    //panic!("asdf");
}
