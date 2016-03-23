use std::fmt;
use std::num;
use std::convert;
use std::cell::RefCell;
use std::rc::Rc;

use mem;
use lcd;
use timer;
use joypad;
use interrupt;

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
    pub tracing: bool,
    halt: bool,
}

impl fmt::Debug for Cpu {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Cpu {{ af:{:04x}({}{}{}{}) bc:{:04x} de:{:04x} \
               hl:{:04x} pc:{:4x} sp:{:4x} cycles:{} }}",
               self.af(),
               if self.f & FLAG_ZERO > 0 { 'z' } else { '.' },
               if self.f & FLAG_SUBTRACT > 0 { 's' } else { '.' },
               if self.f & FLAG_HALF_CARRY > 0 { 'h' } else { '.' },
               if self.f & FLAG_CARRY > 0 { 'c' } else { '.' },
               self.bc(), self.de(), self.hl(),
               self.pc, self.sp, self.cycles)
    }
}

macro_rules! my_log {
    ($_self:ident, $fmt:expr) => {{
        //if $_self.tracing && !($_self.pc >= 0x2ed && $_self.pc <= 0x2f0) {
        if $_self.tracing {
            println!($fmt);
        }
    }};
    ($_self:ident, $fmt:expr, $($arg:tt)*) => {{
        //if $_self.tracing && !($_self.pc >= 0x2ed && $_self.pc <= 0x2f0) {
        if $_self.tracing {
            println!($fmt, $($arg)*);
        }
    }};
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
            tracing: false,
            halt: false,
        }
    }

    fn af(&self) -> u16 {
        return (self.a as u16) << 8 | (self.f as u16);
    }
    fn set_af(&mut self, af: u16) {
        self.a = (af >> 8) as u8;
        self.f = (af & 0xff) as u8;
        self.f &= 0xf0; // the bottom four bits of f are always 0
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

    fn read_u16(&self, mm: &mut mem::MemoryMap, pos: u16) -> u16 {
        return (mm.read(pos + 1) as u16) << 8 | (mm.read(pos) as u16);
    }

    fn add(&mut self, val: u8) {
        let pa = self.a;
        self.a = self.a.wrapping_add(val);
        let a = self.a;
        self.set_zero(a == 0);
        self.set_subtract(false);
        self.set_half_carry((a & 0xf) < (pa & 0xf));
        self.set_carry(a < pa);
    }

    fn adc(&mut self, val: u8) {
        let carry : u8 = if self.carry() { 1 } else { 0 };
        let pa = self.a;
        self.a = self.a.wrapping_add(val).wrapping_add(carry);
        let a = self.a;
        self.set_zero(a == 0);
        self.set_subtract(false);
        self.set_half_carry((pa & 0xf) + (val & 0xf) + carry > 0xf);
        if carry > 0 {
            self.set_carry(a <= pa);
        } else {
            self.set_carry(a < pa);
        }
    }

    fn sub(&mut self, val: u8) {
        let pa = self.a;
        self.a = self.a.wrapping_sub(val);
        let a = self.a;
        self.set_zero(a == 0);
        self.set_subtract(true);
        self.set_half_carry(pa & 0xf < a & 0xf);
        self.set_carry(a > pa);
    }

    fn sbc(&mut self, val: u8) {
        let carry = if self.carry() { 1 } else { 0 };
        let pa = self.a;
        self.a = self.a.wrapping_sub(val).wrapping_sub(carry);
        let a = self.a;
        self.set_zero(a == 0);
        self.set_subtract(true);
        self.set_half_carry((pa & 0xf).wrapping_sub(val & 0xf).wrapping_sub(carry) > 200);
        if carry > 0 {
            self.set_carry(a >= pa);
        } else {
            self.set_carry(a > pa);
        }
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
        let tmp = a.wrapping_sub(val);
        self.set_zero(a == val);
        self.set_subtract(true);
        self.set_half_carry((tmp & 0xf) > (a & 0xf));
        self.set_carry(val > a);
    }

    fn rlc(&mut self, val: u8) -> u8 {
        let carry = val & 0x80;
        let mut newval = val << 1;

        if carry == 0 {
            self.set_carry(false);
        } else {
            newval |= 0x1;
            self.set_carry(true);
        }
        self.set_zero(newval == 0);
        self.set_subtract(false);
        self.set_half_carry(false);

        return newval;
    }

    fn rl(&mut self, val: u8) -> u8 {
        let carry = val & 0x80;
        let mut newval = val << 1;

        if self.carry() {
            newval |= 0x1;
        }
        self.set_zero(newval == 0);
        self.set_subtract(false);
        self.set_half_carry(false);
        self.set_carry(carry != 0);
        return newval;
    }

    fn sla(&mut self, val: u8) -> u8 {
        let carry = val & 0x80;
        let newval = val << 1;

        self.set_zero(newval == 0);
        self.set_subtract(false);
        self.set_half_carry(false);
        self.set_carry(carry != 0);

        return newval;
    }

    fn sra(&mut self, val: u8) -> u8 {
        let carry = val & 0x1;
        let carrytop = val & 0x80;
        let mut newval = val >> 1;

        if carrytop > 0 {
            newval |= 0x80;
        }

        self.set_zero(newval == 0);
        self.set_subtract(false);
        self.set_half_carry(false);
        self.set_carry(carry != 0);

        return newval;
    }

    fn srl(&mut self, val: u8) -> u8 {
        let carry = val & 0x1;
        let newval = val >> 1;

        self.set_zero(newval == 0);
        self.set_subtract(false);
        self.set_half_carry(false);
        self.set_carry(carry != 0);

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
        self.set_subtract(false);
        self.set_half_carry(false);
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
        self.set_subtract(false);
        self.set_half_carry(false);
        self.set_zero(newval == 0);
        return newval;
    }

    fn swap(&mut self, val: u8) -> u8 {
        let top = val >> 4;
        let bottom = val & 0x0f;
        self.set_zero(top == 0 && bottom == 0);
        self.set_subtract(false);
        self.set_half_carry(false);
        self.set_carry(false);
        return bottom << 4 | top;
    }

    fn inc(&mut self, val: u8) -> u8 {
        let newval = val.wrapping_add(1);
        self.set_zero(newval == 0);
        self.set_subtract(false);
        self.set_half_carry((newval & 0xf) == 0);
        return newval;
    }

    fn inc16(&mut self, val: u16) -> u16 {
        val.wrapping_add(1)
    }

    fn dec(&mut self, val: u8) -> u8 {
        let newval = val.wrapping_sub(1);
        self.set_zero(newval == 0);
        self.set_subtract(true);
        self.set_half_carry((newval & 0xf) == 0xf);
        return newval;
    }

    fn dec16(&mut self, val: u16) -> u16 {
        val.wrapping_sub(1)
    }

    fn stack_write_u16(&mut self, mm: &mut mem::MemoryMap, addr: u16) {
        mm.write(self.sp - 1, (addr >> 8) as u8);
        mm.write(self.sp - 2, (addr & 0xff) as u8);
        self.sp -= 2;
    }

    fn stack_read_u16(&mut self, mm: &mut mem::MemoryMap) -> u16 {
        let lower = mm.read(self.sp);
        let upper = mm.read(self.sp + 1);
        self.sp += 2;
        return (upper as u16) << 8 | (lower as u16);
    }

    fn res(&self, bit: u8, reg: u8) -> u8 {
        reg & !(1 << bit)
    }

    fn set(&self, bit: u8, reg: u8) -> u8 {
        reg | (1 << bit)
    }

    fn bit(&mut self, bit: u8, reg: u8) {
        self.set_zero(reg & (1 << bit) == 0);
        self.set_subtract(false);
        self.set_half_carry(true);
    }

    fn add_hl(&mut self, val: u16) {
        let hl = self.hl();
        let newval = hl.wrapping_add(val);
        self.set_subtract(false);
        self.set_half_carry((hl & 0xfff) + (val & 0xfff) > 0xfff);
        self.set_carry(newval < hl);
        self.set_hl(newval);
    }

    fn daa(&mut self) {
        if !self.subtract() {
            if self.carry() || self.a > 0x99 {
                self.a = self.a.wrapping_add(0x60);
                self.set_carry(true);
            }
            if self.half_carry() || (self.a & 0xf) > 0x9 {
                self.a = self.a.wrapping_add(0x06);
                self.set_half_carry(false);
            }
        } else if self.carry() && self.half_carry() {
            self.a = self.a.wrapping_add(0x9a);
            self.set_half_carry(false);
        } else if self.carry() {
            self.a = self.a.wrapping_add(0xa0);
        } else if self.half_carry() {
            self.a = self.a.wrapping_add(0xfa);
            self.set_half_carry(false);
        }
        let a = self.a;
        self.set_zero(a == 0);
    }

    fn handle_cb(&mut self, mm: &mut mem::MemoryMap) -> u32 {
        let opcode = mm.read(self.pc + 1);
        let mut cycles = 0u32;
        //my_log!(self, "opcode={:02x}", opcode);
        match opcode {
            0x00 => { my_log!(self,"rlc b"); let val = self.b; self.b = self.rlc(val); cycles += 8; },
            0x01 => { my_log!(self,"rlc c"); let val = self.c; self.c = self.rlc(val); cycles += 8; },
            0x02 => { my_log!(self,"rlc d"); let val = self.d; self.d = self.rlc(val); cycles += 8; },
            0x03 => { my_log!(self,"rlc e"); let val = self.e; self.e = self.rlc(val); cycles += 8; },
            0x04 => { my_log!(self,"rlc h"); let val = self.h; self.h = self.rlc(val); cycles += 8; },
            0x05 => { my_log!(self,"rlc l"); let val = self.l; self.l = self.rlc(val); cycles += 8; },
            0x06 => { my_log!(self,"rlc (hl)"); let hl = self.hl(); let val = self.rlc(mm.read(hl)); mm.write(hl, val); cycles += 16; },
            0x07 => { my_log!(self,"rlc a"); let val = self.a; self.a = self.rlc(val); cycles += 8; },
            0x08 => { my_log!(self,"rrc b"); let val = self.b; self.b = self.rrc(val); cycles += 8; },
            0x09 => { my_log!(self,"rrc c"); let val = self.c; self.c = self.rrc(val); cycles += 8; },
            0x0a => { my_log!(self,"rrc d"); let val = self.d; self.d = self.rrc(val); cycles += 8; },
            0x0b => { my_log!(self,"rrc e"); let val = self.e; self.e = self.rrc(val); cycles += 8; },
            0x0c => { my_log!(self,"rrc h"); let val = self.h; self.h = self.rrc(val); cycles += 8; },
            0x0d => { my_log!(self,"rrc l"); let val = self.l; self.l = self.rrc(val); cycles += 8; },
            0x0e => { my_log!(self,"rrc (hl)"); let hl = self.hl(); let val = self.rrc(mm.read(hl)); mm.write(hl, val); cycles += 16; },
            0x0f => { my_log!(self,"rrc a"); let val = self.a; self.a = self.rrc(val); cycles += 8; },
            0x10 => { my_log!(self,"rl b"); let val = self.b; self.b = self.rl(val); cycles += 8; },
            0x11 => { my_log!(self,"rl c"); let val = self.c; self.c = self.rl(val); cycles += 8; },
            0x12 => { my_log!(self,"rl d"); let val = self.d; self.d = self.rl(val); cycles += 8; },
            0x13 => { my_log!(self,"rl e"); let val = self.e; self.e = self.rl(val); cycles += 8; },
            0x14 => { my_log!(self,"rl h"); let val = self.h; self.h = self.rl(val); cycles += 8; },
            0x15 => { my_log!(self,"rl l"); let val = self.l; self.l = self.rl(val); cycles += 8; },
            0x16 => { my_log!(self,"rl (hl)"); let hl = self.hl(); let val = self.rl(mm.read(hl)); mm.write(hl, val); cycles += 16; },
            0x17 => { my_log!(self,"rl a"); let val = self.a; self.a = self.rl(val); cycles += 8; },
            0x18 => { my_log!(self,"rr b"); let val = self.b; self.b = self.rr(val); cycles += 8; },
            0x19 => { my_log!(self,"rr c"); let val = self.c; self.c = self.rr(val); cycles += 8; },
            0x1a => { my_log!(self,"rr d"); let val = self.d; self.d = self.rr(val); cycles += 8; },
            0x1b => { my_log!(self,"rr e"); let val = self.e; self.e = self.rr(val); cycles += 8; },
            0x1c => { my_log!(self,"rr h"); let val = self.h; self.h = self.rr(val); cycles += 8; },
            0x1d => { my_log!(self,"rr l"); let val = self.l; self.l = self.rr(val); cycles += 8; },
            0x1e => { my_log!(self,"rr (hl)"); let hl = self.hl(); let val = self.rr(mm.read(hl)); mm.write(hl, val); cycles += 16; },
            0x1f => { my_log!(self,"rr a"); let val = self.a; self.a = self.rr(val); cycles += 8; },
            0x20 => { my_log!(self,"sla b"); let val = self.b; self.b = self.sla(val); cycles += 8; },
            0x21 => { my_log!(self,"sla c"); let val = self.c; self.c = self.sla(val); cycles += 8; },
            0x22 => { my_log!(self,"sla d"); let val = self.d; self.d = self.sla(val); cycles += 8; },
            0x23 => { my_log!(self,"sla e"); let val = self.e; self.e = self.sla(val); cycles += 8; },
            0x24 => { my_log!(self,"sla h"); let val = self.h; self.h = self.sla(val); cycles += 8; },
            0x25 => { my_log!(self,"sla l"); let val = self.l; self.l = self.sla(val); cycles += 8; },
            0x26 => { my_log!(self,"sla (hl)"); let hl = self.hl(); let val = self.sla(mm.read(hl)); mm.write(hl, val); cycles += 16; },
            0x27 => { my_log!(self,"sla a"); let val = self.a; self.a = self.sla(val); cycles += 8; },
            0x28 => { my_log!(self,"sra b"); let val = self.b; self.b = self.sra(val); cycles += 8; },
            0x29 => { my_log!(self,"sra c"); let val = self.c; self.c = self.sra(val); cycles += 8; },
            0x2a => { my_log!(self,"sra d"); let val = self.d; self.d = self.sra(val); cycles += 8; },
            0x2b => { my_log!(self,"sra e"); let val = self.e; self.e = self.sra(val); cycles += 8; },
            0x2c => { my_log!(self,"sra h"); let val = self.h; self.h = self.sra(val); cycles += 8; },
            0x2d => { my_log!(self,"sra l"); let val = self.l; self.l = self.sra(val); cycles += 8; },
            0x2e => { my_log!(self,"sra (hl)"); let hl = self.hl(); let val = self.sra(mm.read(hl)); mm.write(hl, val); cycles += 16; },
            0x2f => { my_log!(self,"sra a"); let val = self.a; self.a = self.sra(val); cycles += 8; },
            0x30 => { my_log!(self,"swap b"); let val = self.b; self.b = self.swap(val); cycles += 8; },
            0x31 => { my_log!(self,"swap c"); let val = self.c; self.c = self.swap(val); cycles += 8; },
            0x32 => { my_log!(self,"swap d"); let val = self.d; self.d = self.swap(val); cycles += 8; },
            0x33 => { my_log!(self,"swap e"); let val = self.e; self.e = self.swap(val); cycles += 8; },
            0x34 => { my_log!(self,"swap h"); let val = self.h; self.h = self.swap(val); cycles += 8; },
            0x35 => { my_log!(self,"swap l"); let val = self.l; self.l = self.swap(val); cycles += 8; },
            0x36 => { my_log!(self,"swap (hl)"); let hl = self.hl(); let val = self.swap(mm.read(hl)); mm.write(hl, val); cycles += 16; },
            0x37 => { my_log!(self,"swap a"); let val = self.a; self.a = self.swap(val); cycles += 8; },
            0x38 => { my_log!(self,"srl b"); let val = self.b; self.b = self.srl(val); cycles += 8; },
            0x39 => { my_log!(self,"srl c"); let val = self.c; self.c = self.srl(val); cycles += 8; },
            0x3a => { my_log!(self,"srl d"); let val = self.d; self.d = self.srl(val); cycles += 8; },
            0x3b => { my_log!(self,"srl e"); let val = self.e; self.e = self.srl(val); cycles += 8; },
            0x3c => { my_log!(self,"srl h"); let val = self.h; self.h = self.srl(val); cycles += 8; },
            0x3d => { my_log!(self,"srl l"); let val = self.l; self.l = self.srl(val); cycles += 8; },
            0x3e => { my_log!(self,"srl (hl)"); let hl = self.hl(); let val = self.srl(mm.read(hl)); mm.write(hl, val); cycles += 16; },
            0x3f => { my_log!(self,"srl a"); let val = self.a; self.a = self.srl(val); cycles += 8; },
            0x40 => { my_log!(self,"bit 0, b"); let val = self.b; self.bit(0, val); cycles += 8; },
            0x41 => { my_log!(self,"bit 0, c"); let val = self.c; self.bit(0, val); cycles += 8; },
            0x42 => { my_log!(self,"bit 0, d"); let val = self.d; self.bit(0, val); cycles += 8; },
            0x43 => { my_log!(self,"bit 0, e"); let val = self.e; self.bit(0, val); cycles += 8; },
            0x44 => { my_log!(self,"bit 0, h"); let val = self.h; self.bit(0, val); cycles += 8; },
            0x45 => { my_log!(self,"bit 0, l"); let val = self.l; self.bit(0, val); cycles += 8; },
            0x46 => { my_log!(self,"bit 0, (hl)"); let hl = self.hl(); self.bit(0, mm.read(hl)); cycles += 8; },
            0x47 => { my_log!(self,"bit 0, a"); let val = self.a; self.bit(0, val); cycles += 8; },
            0x48 => { my_log!(self,"bit 1, b"); let val = self.b; self.bit(1, val); cycles += 8; },
            0x49 => { my_log!(self,"bit 1, c"); let val = self.c; self.bit(1, val); cycles += 8; },
            0x4a => { my_log!(self,"bit 1, d"); let val = self.d; self.bit(1, val); cycles += 8; },
            0x4b => { my_log!(self,"bit 1, e"); let val = self.e; self.bit(1, val); cycles += 8; },
            0x4c => { my_log!(self,"bit 1, h"); let val = self.h; self.bit(1, val); cycles += 8; },
            0x4d => { my_log!(self,"bit 1, l"); let val = self.l; self.bit(1, val); cycles += 8; },
            0x4e => { my_log!(self,"bit 1, (hl)"); let hl = self.hl(); self.bit(1, mm.read(hl)); cycles += 8; },
            0x4f => { my_log!(self,"bit 1, a"); let val = self.a; self.bit(1, val); cycles += 8; },
            0x50 => { my_log!(self,"bit 2, b"); let val = self.b; self.bit(2, val); cycles += 8; },
            0x51 => { my_log!(self,"bit 2, c"); let val = self.c; self.bit(2, val); cycles += 8; },
            0x52 => { my_log!(self,"bit 2, d"); let val = self.d; self.bit(2, val); cycles += 8; },
            0x53 => { my_log!(self,"bit 2, e"); let val = self.e; self.bit(2, val); cycles += 8; },
            0x54 => { my_log!(self,"bit 2, h"); let val = self.h; self.bit(2, val); cycles += 8; },
            0x55 => { my_log!(self,"bit 2, l"); let val = self.l; self.bit(2, val); cycles += 8; },
            0x56 => { my_log!(self,"bit 2, (hl)"); let hl = self.hl(); self.bit(2, mm.read(hl)); cycles += 8; },
            0x57 => { my_log!(self,"bit 2, a"); let val = self.a; self.bit(2, val); cycles += 8; },
            0x58 => { my_log!(self,"bit 3, b"); let val = self.b; self.bit(3, val); cycles += 8; },
            0x59 => { my_log!(self,"bit 3, c"); let val = self.c; self.bit(3, val); cycles += 8; },
            0x5a => { my_log!(self,"bit 3, d"); let val = self.d; self.bit(3, val); cycles += 8; },
            0x5b => { my_log!(self,"bit 3, e"); let val = self.e; self.bit(3, val); cycles += 8; },
            0x5c => { my_log!(self,"bit 3, h"); let val = self.h; self.bit(3, val); cycles += 8; },
            0x5d => { my_log!(self,"bit 3, l"); let val = self.l; self.bit(3, val); cycles += 8; },
            0x5e => { my_log!(self,"bit 3, (hl)"); let hl = self.hl(); self.bit(3, mm.read(hl)); cycles += 8; },
            0x5f => { my_log!(self,"bit 3, a"); let val = self.a; self.bit(3, val); cycles += 8; },
            0x60 => { my_log!(self,"bit 4, b"); let val = self.b; self.bit(4, val); cycles += 8; },
            0x61 => { my_log!(self,"bit 4, c"); let val = self.c; self.bit(4, val); cycles += 8; },
            0x62 => { my_log!(self,"bit 4, d"); let val = self.d; self.bit(4, val); cycles += 8; },
            0x63 => { my_log!(self,"bit 4, e"); let val = self.e; self.bit(4, val); cycles += 8; },
            0x64 => { my_log!(self,"bit 4, h"); let val = self.h; self.bit(4, val); cycles += 8; },
            0x65 => { my_log!(self,"bit 4, l"); let val = self.l; self.bit(4, val); cycles += 8; },
            0x66 => { my_log!(self,"bit 4, (hl)"); let hl = self.hl(); self.bit(4, mm.read(hl)); cycles += 8; },
            0x67 => { my_log!(self,"bit 4, a"); let val = self.a; self.bit(4, val); cycles += 8; },
            0x68 => { my_log!(self,"bit 5, b"); let val = self.b; self.bit(5, val); cycles += 8; },
            0x69 => { my_log!(self,"bit 5, c"); let val = self.c; self.bit(5, val); cycles += 8; },
            0x6a => { my_log!(self,"bit 5, d"); let val = self.d; self.bit(5, val); cycles += 8; },
            0x6b => { my_log!(self,"bit 5, e"); let val = self.e; self.bit(5, val); cycles += 8; },
            0x6c => { my_log!(self,"bit 5, h"); let val = self.h; self.bit(5, val); cycles += 8; },
            0x6d => { my_log!(self,"bit 5, l"); let val = self.l; self.bit(5, val); cycles += 8; },
            0x6e => { my_log!(self,"bit 5, (hl)"); let hl = self.hl(); self.bit(5, mm.read(hl)); cycles += 8; },
            0x6f => { my_log!(self,"bit 5, a"); let val = self.a; self.bit(5, val); cycles += 8; },
            0x70 => { my_log!(self,"bit 6, b"); let val = self.b; self.bit(6, val); cycles += 8; },
            0x71 => { my_log!(self,"bit 6, c"); let val = self.c; self.bit(6, val); cycles += 8; },
            0x72 => { my_log!(self,"bit 6, d"); let val = self.d; self.bit(6, val); cycles += 8; },
            0x73 => { my_log!(self,"bit 6, e"); let val = self.e; self.bit(6, val); cycles += 8; },
            0x74 => { my_log!(self,"bit 6, h"); let val = self.h; self.bit(6, val); cycles += 8; },
            0x75 => { my_log!(self,"bit 6, l"); let val = self.l; self.bit(6, val); cycles += 8; },
            0x76 => { my_log!(self,"bit 6, (hl)"); let hl = self.hl(); self.bit(6, mm.read(hl)); cycles += 8; },
            0x77 => { my_log!(self,"bit 6, a"); let val = self.a; self.bit(6, val); cycles += 8; },
            0x78 => { my_log!(self,"bit 7, b"); let val = self.b; self.bit(7, val); cycles += 8; },
            0x79 => { my_log!(self,"bit 7, c"); let val = self.c; self.bit(7, val); cycles += 8; },
            0x7a => { my_log!(self,"bit 7, d"); let val = self.d; self.bit(7, val); cycles += 8; },
            0x7b => { my_log!(self,"bit 7, e"); let val = self.e; self.bit(7, val); cycles += 8; },
            0x7c => { my_log!(self,"bit 7, h"); let val = self.h; self.bit(7, val); cycles += 8; },
            0x7d => { my_log!(self,"bit 7, l"); let val = self.l; self.bit(7, val); cycles += 8; },
            0x7e => { my_log!(self,"bit 7, (hl)"); let hl = self.hl(); self.bit(7, mm.read(hl)); cycles += 8; },
            0x7f => { my_log!(self,"bit 7, a"); let val = self.a; self.bit(7, val); cycles += 8; },
            0x80 => { my_log!(self,"res 0, b"); let val = self.b; self.b = self.res(0, val); cycles += 8; },
            0x81 => { my_log!(self,"res 0, c"); let val = self.c; self.c = self.res(0, val); cycles += 8; },
            0x82 => { my_log!(self,"res 0, d"); let val = self.d; self.d = self.res(0, val); cycles += 8; },
            0x83 => { my_log!(self,"res 0, e"); let val = self.e; self.e = self.res(0, val); cycles += 8; },
            0x84 => { my_log!(self,"res 0, h"); let val = self.h; self.h = self.res(0, val); cycles += 8; },
            0x85 => { my_log!(self,"res 0, l"); let val = self.l; self.l = self.res(0, val); cycles += 8; },
            0x86 => { my_log!(self,"res 0, (hl)"); let hl = self.hl(); let val = self.res(0, mm.read(hl)); mm.write(hl, val); cycles += 16; },
            0x87 => { my_log!(self,"res 0, a"); let val = self.a; self.a = self.res(0, val); cycles += 8; },
            0x88 => { my_log!(self,"res 1, b"); let val = self.b; self.b = self.res(1, val); cycles += 8; },
            0x89 => { my_log!(self,"res 1, c"); let val = self.c; self.c = self.res(1, val); cycles += 8; },
            0x8a => { my_log!(self,"res 1, d"); let val = self.d; self.d = self.res(1, val); cycles += 8; },
            0x8b => { my_log!(self,"res 1, e"); let val = self.e; self.e = self.res(1, val); cycles += 8; },
            0x8c => { my_log!(self,"res 1, h"); let val = self.h; self.h = self.res(1, val); cycles += 8; },
            0x8d => { my_log!(self,"res 1, l"); let val = self.l; self.l = self.res(1, val); cycles += 8; },
            0x8e => { my_log!(self,"res 1, (hl)"); let hl = self.hl(); let val = self.res(1, mm.read(hl)); mm.write(hl, val); cycles += 16; },
            0x8f => { my_log!(self,"res 1, a"); let val = self.a; self.a = self.res(1, val); cycles += 8; },
            0x90 => { my_log!(self,"res 2, b"); let val = self.b; self.b = self.res(2, val); cycles += 8; },
            0x91 => { my_log!(self,"res 2, c"); let val = self.c; self.c = self.res(2, val); cycles += 8; },
            0x92 => { my_log!(self,"res 2, d"); let val = self.d; self.d = self.res(2, val); cycles += 8; },
            0x93 => { my_log!(self,"res 2, e"); let val = self.e; self.e = self.res(2, val); cycles += 8; },
            0x94 => { my_log!(self,"res 2, h"); let val = self.h; self.h = self.res(2, val); cycles += 8; },
            0x95 => { my_log!(self,"res 2, l"); let val = self.l; self.l = self.res(2, val); cycles += 8; },
            0x96 => { my_log!(self,"res 2, (hl)"); let hl = self.hl(); let val = self.res(2, mm.read(hl)); mm.write(hl, val); cycles += 16; },
            0x97 => { my_log!(self,"res 2, a"); let val = self.a; self.a = self.res(2, val); cycles += 8; },
            0x98 => { my_log!(self,"res 3, b"); let val = self.b; self.b = self.res(3, val); cycles += 8; },
            0x99 => { my_log!(self,"res 3, c"); let val = self.c; self.c = self.res(3, val); cycles += 8; },
            0x9a => { my_log!(self,"res 3, d"); let val = self.d; self.d = self.res(3, val); cycles += 8; },
            0x9b => { my_log!(self,"res 3, e"); let val = self.e; self.e = self.res(3, val); cycles += 8; },
            0x9c => { my_log!(self,"res 3, h"); let val = self.h; self.h = self.res(3, val); cycles += 8; },
            0x9d => { my_log!(self,"res 3, l"); let val = self.l; self.l = self.res(3, val); cycles += 8; },
            0x9e => { my_log!(self,"res 3, (hl)"); let hl = self.hl(); let val = self.res(3, mm.read(hl)); mm.write(hl, val); cycles += 16; },
            0x9f => { my_log!(self,"res 3, a"); let val = self.a; self.a = self.res(3, val); cycles += 8; },
            0xa0 => { my_log!(self,"res 4, b"); let val = self.b; self.b = self.res(4, val); cycles += 8; },
            0xa1 => { my_log!(self,"res 4, c"); let val = self.c; self.c = self.res(4, val); cycles += 8; },
            0xa2 => { my_log!(self,"res 4, d"); let val = self.d; self.d = self.res(4, val); cycles += 8; },
            0xa3 => { my_log!(self,"res 4, e"); let val = self.e; self.e = self.res(4, val); cycles += 8; },
            0xa4 => { my_log!(self,"res 4, h"); let val = self.h; self.h = self.res(4, val); cycles += 8; },
            0xa5 => { my_log!(self,"res 4, l"); let val = self.l; self.l = self.res(4, val); cycles += 8; },
            0xa6 => { my_log!(self,"res 4, (hl)"); let hl = self.hl(); let val = self.res(4, mm.read(hl)); mm.write(hl, val); cycles += 16; },
            0xa7 => { my_log!(self,"res 4, a"); let val = self.a; self.a = self.res(4, val); cycles += 8; },
            0xa8 => { my_log!(self,"res 5, b"); let val = self.b; self.b = self.res(5, val); cycles += 8; },
            0xa9 => { my_log!(self,"res 5, c"); let val = self.c; self.c = self.res(5, val); cycles += 8; },
            0xaa => { my_log!(self,"res 5, d"); let val = self.d; self.d = self.res(5, val); cycles += 8; },
            0xab => { my_log!(self,"res 5, e"); let val = self.e; self.e = self.res(5, val); cycles += 8; },
            0xac => { my_log!(self,"res 5, h"); let val = self.h; self.h = self.res(5, val); cycles += 8; },
            0xad => { my_log!(self,"res 5, l"); let val = self.l; self.l = self.res(5, val); cycles += 8; },
            0xae => { my_log!(self,"res 5, (hl)"); let hl = self.hl(); let val = self.res(5, mm.read(hl)); mm.write(hl, val); cycles += 16; },
            0xaf => { my_log!(self,"res 5, a"); let val = self.a; self.a = self.res(5, val); cycles += 8; },
            0xb0 => { my_log!(self,"res 6, b"); let val = self.b; self.b = self.res(6, val); cycles += 8; },
            0xb1 => { my_log!(self,"res 6, c"); let val = self.c; self.c = self.res(6, val); cycles += 8; },
            0xb2 => { my_log!(self,"res 6, d"); let val = self.d; self.d = self.res(6, val); cycles += 8; },
            0xb3 => { my_log!(self,"res 6, e"); let val = self.e; self.e = self.res(6, val); cycles += 8; },
            0xb4 => { my_log!(self,"res 6, h"); let val = self.h; self.h = self.res(6, val); cycles += 8; },
            0xb5 => { my_log!(self,"res 6, l"); let val = self.l; self.l = self.res(6, val); cycles += 8; },
            0xb6 => { my_log!(self,"res 6, (hl)"); let hl = self.hl(); let val = self.res(6, mm.read(hl)); mm.write(hl, val); cycles += 16; },
            0xb7 => { my_log!(self,"res 6, a"); let val = self.a; self.a = self.res(6, val); cycles += 8; },
            0xb8 => { my_log!(self,"res 7, b"); let val = self.b; self.b = self.res(7, val); cycles += 8; },
            0xb9 => { my_log!(self,"res 7, c"); let val = self.c; self.c = self.res(7, val); cycles += 8; },
            0xba => { my_log!(self,"res 7, d"); let val = self.d; self.d = self.res(7, val); cycles += 8; },
            0xbb => { my_log!(self,"res 7, e"); let val = self.e; self.e = self.res(7, val); cycles += 8; },
            0xbc => { my_log!(self,"res 7, h"); let val = self.h; self.h = self.res(7, val); cycles += 8; },
            0xbd => { my_log!(self,"res 7, l"); let val = self.l; self.l = self.res(7, val); cycles += 8; },
            0xbe => { my_log!(self,"res 7, (hl)"); let hl = self.hl(); let val = self.res(7, mm.read(hl)); mm.write(hl, val); cycles += 16; },
            0xbf => { my_log!(self,"res 7, a"); let val = self.a; self.a = self.res(7, val); cycles += 8; },
            0xc0 => { my_log!(self,"set 0, b"); let val = self.b; self.b = self.set(0, val); cycles += 8; },
            0xc1 => { my_log!(self,"set 0, c"); let val = self.c; self.c = self.set(0, val); cycles += 8; },
            0xc2 => { my_log!(self,"set 0, d"); let val = self.d; self.d = self.set(0, val); cycles += 8; },
            0xc3 => { my_log!(self,"set 0, e"); let val = self.e; self.e = self.set(0, val); cycles += 8; },
            0xc4 => { my_log!(self,"set 0, h"); let val = self.h; self.h = self.set(0, val); cycles += 8; },
            0xc5 => { my_log!(self,"set 0, l"); let val = self.l; self.l = self.set(0, val); cycles += 8; },
            0xc6 => { my_log!(self,"set 0, (hl)"); let hl = self.hl(); let val = self.set(0, mm.read(hl)); mm.write(hl, val); cycles += 16; },
            0xc7 => { my_log!(self,"set 0, a"); let val = self.a; self.a = self.set(0, val); cycles += 8; },
            0xc8 => { my_log!(self,"set 1, b"); let val = self.b; self.b = self.set(1, val); cycles += 8; },
            0xc9 => { my_log!(self,"set 1, c"); let val = self.c; self.c = self.set(1, val); cycles += 8; },
            0xca => { my_log!(self,"set 1, d"); let val = self.d; self.d = self.set(1, val); cycles += 8; },
            0xcb => { my_log!(self,"set 1, e"); let val = self.e; self.e = self.set(1, val); cycles += 8; },
            0xcc => { my_log!(self,"set 1, h"); let val = self.h; self.h = self.set(1, val); cycles += 8; },
            0xcd => { my_log!(self,"set 1, l"); let val = self.l; self.l = self.set(1, val); cycles += 8; },
            0xce => { my_log!(self,"set 1, (hl)"); let hl = self.hl(); let val = self.set(1, mm.read(hl)); mm.write(hl, val); cycles += 16; },
            0xcf => { my_log!(self,"set 1, a"); let val = self.a; self.a = self.set(1, val); cycles += 8; },
            0xd0 => { my_log!(self,"set 2, b"); let val = self.b; self.b = self.set(2, val); cycles += 8; },
            0xd1 => { my_log!(self,"set 2, c"); let val = self.c; self.c = self.set(2, val); cycles += 8; },
            0xd2 => { my_log!(self,"set 2, d"); let val = self.d; self.d = self.set(2, val); cycles += 8; },
            0xd3 => { my_log!(self,"set 2, e"); let val = self.e; self.e = self.set(2, val); cycles += 8; },
            0xd4 => { my_log!(self,"set 2, h"); let val = self.h; self.h = self.set(2, val); cycles += 8; },
            0xd5 => { my_log!(self,"set 2, l"); let val = self.l; self.l = self.set(2, val); cycles += 8; },
            0xd6 => { my_log!(self,"set 2, (hl)"); let hl = self.hl(); let val = self.set(2, mm.read(hl)); mm.write(hl, val); cycles += 16; },
            0xd7 => { my_log!(self,"set 2, a"); let val = self.a; self.a = self.set(2, val); cycles += 8; },
            0xd8 => { my_log!(self,"set 3, b"); let val = self.b; self.b = self.set(3, val); cycles += 8; },
            0xd9 => { my_log!(self,"set 3, c"); let val = self.c; self.c = self.set(3, val); cycles += 8; },
            0xda => { my_log!(self,"set 3, d"); let val = self.d; self.d = self.set(3, val); cycles += 8; },
            0xdb => { my_log!(self,"set 3, e"); let val = self.e; self.e = self.set(3, val); cycles += 8; },
            0xdc => { my_log!(self,"set 3, h"); let val = self.h; self.h = self.set(3, val); cycles += 8; },
            0xdd => { my_log!(self,"set 3, l"); let val = self.l; self.l = self.set(3, val); cycles += 8; },
            0xde => { my_log!(self,"set 3, (hl)"); let hl = self.hl(); let val = self.set(3, mm.read(hl)); mm.write(hl, val); cycles += 16; },
            0xdf => { my_log!(self,"set 3, a"); let val = self.a; self.a = self.set(3, val); cycles += 8; },
            0xe0 => { my_log!(self,"set 4, b"); let val = self.b; self.b = self.set(4, val); cycles += 8; },
            0xe1 => { my_log!(self,"set 4, c"); let val = self.c; self.c = self.set(4, val); cycles += 8; },
            0xe2 => { my_log!(self,"set 4, d"); let val = self.d; self.d = self.set(4, val); cycles += 8; },
            0xe3 => { my_log!(self,"set 4, e"); let val = self.e; self.e = self.set(4, val); cycles += 8; },
            0xe4 => { my_log!(self,"set 4, h"); let val = self.h; self.h = self.set(4, val); cycles += 8; },
            0xe5 => { my_log!(self,"set 4, l"); let val = self.l; self.l = self.set(4, val); cycles += 8; },
            0xe6 => { my_log!(self,"set 4, (hl)"); let hl = self.hl(); let val = self.set(4, mm.read(hl)); mm.write(hl, val); cycles += 16; },
            0xe7 => { my_log!(self,"set 4, a"); let val = self.a; self.a = self.set(4, val); cycles += 8; },
            0xe8 => { my_log!(self,"set 5, b"); let val = self.b; self.b = self.set(5, val); cycles += 8; },
            0xe9 => { my_log!(self,"set 5, c"); let val = self.c; self.c = self.set(5, val); cycles += 8; },
            0xea => { my_log!(self,"set 5, d"); let val = self.d; self.d = self.set(5, val); cycles += 8; },
            0xeb => { my_log!(self,"set 5, e"); let val = self.e; self.e = self.set(5, val); cycles += 8; },
            0xec => { my_log!(self,"set 5, h"); let val = self.h; self.h = self.set(5, val); cycles += 8; },
            0xed => { my_log!(self,"set 5, l"); let val = self.l; self.l = self.set(5, val); cycles += 8; },
            0xee => { my_log!(self,"set 5, (hl)"); let hl = self.hl(); let val = self.set(5, mm.read(hl)); mm.write(hl, val); cycles += 16; },
            0xef => { my_log!(self,"set 5, a"); let val = self.a; self.a = self.set(5, val); cycles += 8; },
            0xf0 => { my_log!(self,"set 6, b"); let val = self.b; self.b = self.set(6, val); cycles += 8; },
            0xf1 => { my_log!(self,"set 6, c"); let val = self.c; self.c = self.set(6, val); cycles += 8; },
            0xf2 => { my_log!(self,"set 6, d"); let val = self.d; self.d = self.set(6, val); cycles += 8; },
            0xf3 => { my_log!(self,"set 6, e"); let val = self.e; self.e = self.set(6, val); cycles += 8; },
            0xf4 => { my_log!(self,"set 6, h"); let val = self.h; self.h = self.set(6, val); cycles += 8; },
            0xf5 => { my_log!(self,"set 6, l"); let val = self.l; self.l = self.set(6, val); cycles += 8; },
            0xf6 => { my_log!(self,"set 6, (hl)"); let hl = self.hl(); let val = self.set(6, mm.read(hl)); mm.write(hl, val); cycles += 16; },
            0xf7 => { my_log!(self,"set 6, a"); let val = self.a; self.a = self.set(6, val); cycles += 8; },
            0xf8 => { my_log!(self,"set 7, b"); let val = self.b; self.b = self.set(7, val); cycles += 8; },
            0xf9 => { my_log!(self,"set 7, c"); let val = self.c; self.c = self.set(7, val); cycles += 8; },
            0xfa => { my_log!(self,"set 7, d"); let val = self.d; self.d = self.set(7, val); cycles += 8; },
            0xfb => { my_log!(self,"set 7, e"); let val = self.e; self.e = self.set(7, val); cycles += 8; },
            0xfc => { my_log!(self,"set 7, h"); let val = self.h; self.h = self.set(7, val); cycles += 8; },
            0xfd => { my_log!(self,"set 7, l"); let val = self.l; self.l = self.set(7, val); cycles += 8; },
            0xfe => { my_log!(self,"set 7, (hl)"); let hl = self.hl(); let val = self.set(7, mm.read(hl)); mm.write(hl, val); cycles += 16; },
            0xff => { my_log!(self,"set 7, a"); let val = self.a; self.a = self.set(7, val); cycles += 8; },
            _ => { panic!("bad cb opcode {:02x}", opcode); }
        }
        return cycles
    }

    fn service_interrupt(&mut self, mm: &mut mem::MemoryMap, addr: u16) {
        self.halt = false;
        let pc = self.pc;
        self.stack_write_u16(mm, pc);
        self.pc = addr;
    }

    fn service_interrupts(&mut self, mm: &mut mem::MemoryMap) {
        if mm.interrupt_triggered(interrupt::INTERRUPT_VBLANK) {
            my_log!(self,"interrupt vblank");
            self.service_interrupt(mm, 0x40);
        }
        if mm.interrupt_triggered(interrupt::INTERRUPT_LCD_STAT) {
            my_log!(self,"interrupt lcd stat");
            self.service_interrupt(mm, 0x48);
        }
        if mm.interrupt_triggered(interrupt::INTERRUPT_TIMER) {
            my_log!(self,"interrupt timer");
            self.service_interrupt(mm, 0x50);
        }
        if mm.interrupt_triggered(interrupt::INTERRUPT_SERIAL) {
            my_log!(self,"interrupt serial");
            self.service_interrupt(mm, 0x58);
        }
        if mm.interrupt_triggered(interrupt::INTERRUPT_JOYPAD) {
            my_log!(self,"interrupt joypad");
            self.service_interrupt(mm, 0x60);
        }
    }

    pub fn run(&mut self, mm: &mut mem::MemoryMap) -> u32 {
        let mut pc = self.pc;
        if self.tracing {
            print!("{:?} ", self);
        }
        if self.halt {
            self.cycles += 16;
            self.service_interrupts(mm);
            return self.cycles;
        }

        match mm.read(pc) {
            0x00 => {
                my_log!(self,"nop");
                self.cycles += 4;
                pc += 1;
            },
            0x01 => {
                let val = self.read_u16(mm, pc + 1);
                my_log!(self,"ld bc, ${:04x}", val);
                self.set_bc(val);
                self.cycles += 12;
                pc += 3;
            },
            0x02 => {
                my_log!(self,"ld (bc), a");
                mm.write(self.bc(), self.a);
                self.cycles += 8;
                pc += 1;
            },
            0x03 => {
                my_log!(self,"inc bc");
                let bc = self.bc();
                let inc = self.inc16(bc);
                self.set_bc(inc);
                self.cycles += 8;
                pc += 1;
            },
            0x04 => {
                my_log!(self,"inc b");
                let b = self.b;
                self.b = self.inc(b);
                self.cycles += 4;
                pc += 1;
            },
            0x05 => {
                my_log!(self,"dec b");
                let b = self.b;
                self.b = self.dec(b);
                self.cycles += 4;
                pc += 1;
            },
            0x06 => {
                let val = mm.read(pc + 1);
                my_log!(self,"ld b, ${:02x}", val);
                self.b = val;
                self.cycles += 8;
                pc += 2;
            },
            0x07 => {
                my_log!(self,"rlca");
                let val = self.a;
                self.a = self.rlc(val);
                self.set_zero(false);
                self.cycles += 4;
                pc += 1;
            },
            0x08 => {
                let val = self.read_u16(mm, pc + 1);
                trace!("ld (${:04x}), sp", val);
                mm.write(val + 1, (self.sp >> 8) as u8);
                mm.write(val, (self.sp & 0xff) as u8);
                self.cycles += 20;
                pc += 3;
            },
            0x09 => {
                my_log!(self,"add hl, bc");
                let bc = self.bc();
                self.add_hl(bc);
                self.cycles += 8;
                pc += 1;
            },
            0x0a => {
                my_log!(self,"ld a, (bc)");
                self.a = mm.read(self.bc());
                self.cycles += 8;
                pc += 1;
            },
            0x0b => {
                my_log!(self,"dec bc");
                let bc = self.bc();
                let dec = self.dec16(bc);
                self.set_bc(dec);
                self.cycles += 8;
                pc += 1;
            },
            0x0c => {
                my_log!(self,"inc c");
                let c = self.c;
                self.c = self.inc(c);
                self.cycles += 4;
                pc += 1;
            },
            0x0d => {
                my_log!(self,"dec c");
                let c = self.c;
                self.c = self.dec(c);
                self.cycles += 4;
                pc += 1;
            },
            0x0e => {
                let val = mm.read(pc + 1);
                my_log!(self,"ld c, ${:02x}", val);
                self.c = val;
                self.cycles += 8;
                pc += 2;
            },
            0x0f => {
                my_log!(self,"rrca");
                let a = self.a;
                self.a = self.rrc(a);
                self.set_zero(false);
                self.cycles += 4;
                pc += 1;
            },
            0x10 => {
                panic!("stop");
                // TODO
                self.cycles += 4;
                pc += 2;
            },
            0x11 => {
                let val = self.read_u16(mm, pc + 1);
                my_log!(self,"ld de, ${:04x}", val);
                self.set_de(val);
                self.cycles += 12;
                pc += 3;
            },
            0x12 => {
                my_log!(self,"ld (de), a");
                mm.write(self.de(), self.a);
                self.cycles += 8;
                pc += 1;
            },
            0x13 => {
                my_log!(self,"inc de");
                let de = self.de();
                let inc = self.inc16(de);
                self.set_de(inc);
                self.cycles += 8;
                pc += 1;
            },
            0x14 => {
                my_log!(self,"inc d");
                let d = self.d;
                self.d = self.inc(d);
                self.cycles += 4;
                pc += 1;
            },
            0x15 => {
                my_log!(self,"dec d");
                let d = self.d;
                self.d = self.dec(d);
                self.cycles += 4;
                pc += 1;
            },
            0x16 => {
                let val = mm.read(pc + 1);
                my_log!(self,"ld d, ${:02x}", val);
                self.d = val;
                self.cycles += 8;
                pc += 2;
            },
            0x17 => {
                my_log!(self,"rla");
                let a = self.a;
                self.a = self.rl(a);
                self.set_zero(false);
                self.cycles += 4;
                pc += 1;
            },
            0x18 => {
                let val = mm.read(pc + 1) as i8;
                my_log!(self,"jr ${:02x}", val);
                pc = ((pc as isize) + (val as isize)) as u16;
                self.cycles += 12;
                pc += 2;
            },
            0x19 => {
                my_log!(self,"add hl, de");
                let de = self.de();
                self.add_hl(de);
                self.cycles += 8;
                pc += 1;
            },
            0x1a => {
                my_log!(self,"ld a, (de)");
                self.a = mm.read(self.de());
                self.cycles += 8;
                pc += 1;
            },
            0x1b => {
                my_log!(self,"dec de");
                let de = self.de();
                let dec = self.dec16(de);
                self.set_de(dec);
                self.cycles += 8;
                pc += 1;
            },
            0x1c => {
                my_log!(self,"inc e");
                let e = self.e;
                self.e = self.inc(e);
                self.cycles += 4;
                pc += 1;
            },
            0x1d => {
                my_log!(self,"dec e");
                let e = self.e;
                self.e = self.dec(e);
                self.cycles += 4;
                pc += 1;
            },
            0x1e => {
                let val = mm.read(pc + 1);
                my_log!(self,"ld e, ${:02x}", val);
                self.e = val;
                self.cycles += 8;
                pc += 2;
            },
            0x1f => {
                my_log!(self,"rra");
                let a = self.a;
                self.a = self.rr(a);
                self.set_zero(false);
                self.cycles += 4;
                pc += 1;
            },
            0x20 => {
                let val = mm.read(pc + 1) as i8;
                my_log!(self,"jr nz, #{}", val);
                if !self.zero() {
                    pc = ((pc as isize) + (val as isize)) as u16;
                    self.cycles += 12;
                } else {
                    self.cycles += 8;
                }
                pc += 2;
            },
            0x21 => {
                let val = self.read_u16(mm, pc + 1);
                my_log!(self,"ld hl, ${:04x}", val);
                self.set_hl(val);
                self.cycles += 12;
                pc += 3;
            },
            0x22 => {
                my_log!(self,"ld (hl+), a");
                let hl = self.hl();
                mm.write(hl, self.a);
                self.set_hl(hl.wrapping_add(1));
                self.cycles += 8;
                pc += 1;
            },
            0x23 => {
                my_log!(self,"inc hl");
                let hl = self.hl();
                let inc = self.inc16(hl);
                self.set_hl(inc);
                self.cycles += 8;
                pc += 1;
            },
            0x24 => {
                my_log!(self,"inc h");
                let h = self.h;
                self.h = self.inc(h);
                self.cycles += 4;
                pc += 1;
            },
            0x25 => {
                my_log!(self,"dec h");
                let h = self.h;
                self.h = self.dec(h);
                self.cycles += 4;
                pc += 1;
            },
            0x26 => {
                let val = mm.read(pc + 1);
                my_log!(self,"ld h, ${:02x}", val);
                self.h = val;
                self.cycles += 8;
                pc += 2;
            },
            0x27 => {
                my_log!(self,"daa");
                self.daa();
                self.cycles += 4;
                pc += 1;
            },
            0x28 => {
                let val = mm.read(pc + 1) as i8;
                my_log!(self,"jr z, #{}", val);
                if self.zero() {
                    pc = ((pc as isize) + (val as isize)) as u16;
                    self.cycles += 12;
                } else {
                    self.cycles += 8;
                }
                pc += 2;
            },
            0x29 => {
                my_log!(self,"add hl, hl");
                let hl = self.hl();
                self.add_hl(hl);
                self.cycles += 8;
                pc += 1;
            },
            0x2a => {
                my_log!(self,"ld a, (hl+)");
                let hl = self.hl();
                self.a = mm.read(hl);
                let inc = self.inc16(hl);
                self.set_hl(inc);
                self.cycles += 8;
                pc += 1;
            },
            0x2b => {
                my_log!(self,"dec hl");
                let hl = self.hl();
                let dec = self.dec16(hl);
                self.set_hl(dec);
                self.cycles += 8;
                pc += 1;
            },
            0x2c => {
                my_log!(self,"inc l");
                let l = self.l;
                self.l = self.inc(l);
                self.cycles += 4;
                pc += 1;
            },
            0x2d => {
                my_log!(self,"dec l");
                let l = self.l;
                self.l = self.dec(l);
                self.cycles += 4;
                pc += 1;
            },
            0x2e => {
                let val = mm.read(pc + 1);
                my_log!(self,"ld l, ${:02x}", val);
                self.l = val;
                self.cycles += 8;
                pc += 2;
            },
            0x2f => {
                my_log!(self,"cpl");
                self.a = !self.a;
                let a = self.a;
                self.set_subtract(true);
                self.set_half_carry(true);
                self.cycles += 4;
                pc += 1;
            },
            0x30 => {
                let val = mm.read(pc + 1) as i8;
                my_log!(self,"jr nc, #{}", val);
                if !self.carry() {
                    pc = ((pc as isize) + (val as isize)) as u16;
                    self.cycles += 12;
                } else {
                    self.cycles += 8;
                }
                pc += 2;
            },
            0x31 => {
                let val = self.read_u16(mm, pc + 1);
                my_log!(self,"ld sp, ${:04x}", val);
                self.sp = val;
                self.cycles += 12;
                pc += 3;
            },
            0x32 => {
                my_log!(self,"ld (hl-), a");
                let hl = self.hl();
                mm.write(hl, self.a);
                let dec = self.dec16(hl);
                self.set_hl(dec);
                self.cycles += 8;
                pc += 1;
            },
            0x33 => {
                my_log!(self,"inc sp");
                println!("old sp = {:04x}", self.sp);
                self.sp = self.sp.wrapping_add(1);
                println!("new sp = {:04x}", self.sp);
                self.cycles += 8;
                pc += 1;
            },
            0x34 => {
                my_log!(self,"inc (hl)");
                let hl = self.hl();
                let val = mm.read(hl);
                let newval = self.inc(val);
                mm.write(hl, newval);
                self.cycles += 12;
                pc += 1;
            },
            0x35 => {
                my_log!(self,"dec (hl)");
                let hl = self.hl();
                let val = mm.read(hl);
                let newval = self.dec(val);
                mm.write(hl, newval);
                self.cycles += 12;
                pc += 1;
            },
            0x36 => {
                let val = mm.read(pc + 1);
                my_log!(self,"ld (hl), ${:02x}", val);
                mm.write(self.hl(), val);
                self.cycles += 12;
                pc += 2;
            },
            0x37 => {
                my_log!(self,"scf");
                self.set_subtract(false);
                self.set_half_carry(false);
                self.set_carry(true);
                self.cycles += 4;
                pc += 1;
            },
            0x38 => {
                let val = mm.read(pc + 1) as i8;
                my_log!(self,"jr c, #{}", val);
                if self.carry() {
                    pc = ((pc as isize) + (val as isize)) as u16;
                    self.cycles += 12;
                } else {
                    self.cycles += 8;
                }
                pc += 2;
            },
            0x39 => {
                my_log!(self,"add hl, sp");
                let sp = self.sp;
                self.add_hl(sp);
                self.cycles += 8;
                pc += 2;
            },
            0x3a => {
                my_log!(self,"ld a, (hl-)");
                self.a = mm.read(self.hl());
                let hl = self.hl();
                self.set_hl(hl.wrapping_sub(1));
                self.cycles += 8;
                pc += 1;
            },
            0x3b => {
                my_log!(self,"dec sp");
                self.sp = self.sp.wrapping_sub(1);
                self.cycles += 8;
                pc += 2;
            },
            0x3c => {
                my_log!(self,"inc a");
                let a = self.a;
                self.a = self.inc(a);
                self.cycles += 4;
                pc += 1;
            },
            0x3d => {
                my_log!(self,"dec a");
                let a = self.a;
                self.a = self.dec(a);
                self.cycles += 4;
                pc += 1;
            },
            0x3e => {
                let val = mm.read(pc + 1);
                my_log!(self,"ld a, ${:02x}", val);
                self.a = val;
                self.cycles += 8;
                pc += 2;
            },
            0x3f => {
                my_log!(self,"ccf");
                let c = self.carry();
                self.set_subtract(false);
                self.set_half_carry(false);
                self.set_carry(!c);
                self.cycles += 4;
                pc += 1;
            },
            0x40 => {
                my_log!(self,"ld b, b");
                self.b = self.b;
                self.cycles += 4;
                pc += 1;
            },
            0x41 => {
                my_log!(self,"ld b, c");
                self.b = self.c;
                self.cycles += 4;
                pc += 1;
            },
            0x42 => {
                my_log!(self,"ld b, d");
                self.b = self.d;
                self.cycles += 4;
                pc += 1;
            },
            0x43 => {
                my_log!(self,"ld b, e");
                self.b = self.e;
                self.cycles += 4;
                pc += 1;
            },
            0x44 => {
                my_log!(self,"ld b, h");
                self.b = self.h;
                self.cycles += 4;
                pc += 1;
            },
            0x45 => {
                my_log!(self,"ld b, l");
                self.b = self.l;
                self.cycles += 4;
                pc += 1;
            },
            0x46 => {
                my_log!(self,"ld b, (hl)");
                self.b = mm.read(self.hl());
                self.cycles += 8;
                pc += 1;
            },
            0x47 => {
                my_log!(self,"ld b, a");
                self.b = self.a;
                self.cycles += 4;
                pc += 1;
            },
            0x48 => {
                my_log!(self,"ld c, b");
                self.c = self.b;
                self.cycles += 4;
                pc += 1;
            },
            0x49 => {
                my_log!(self,"ld c, c");
                self.c = self.c;
                self.cycles += 4;
                pc += 1;
            },
            0x4a => {
                my_log!(self,"ld c, d");
                self.c = self.d;
                self.cycles += 4;
                pc += 1;
            },
            0x4b => {
                my_log!(self,"ld c, e");
                self.c = self.e;
                self.cycles += 4;
                pc += 1;
            },
            0x4c => {
                my_log!(self,"ld c, h");
                self.c = self.h;
                self.cycles += 4;
                pc += 1;
            },
            0x4d => {
                my_log!(self,"ld c, l");
                self.c = self.l;
                self.cycles += 4;
                pc += 1;
            },
            0x4e => {
                my_log!(self,"ld c, (hl)");
                self.c = mm.read(self.hl());
                self.cycles += 8;
                pc += 1;
            },
            0x4f => {
                my_log!(self,"ld c, a");
                self.c = self.a;
                self.cycles += 4;
                pc += 1;
            },
            0x50 => {
                my_log!(self,"ld d, b");
                self.d = self.b;
                self.cycles += 4;
                pc += 1;
            },
            0x51 => {
                my_log!(self,"ld d, c");
                self.d = self.c;
                self.cycles += 4;
                pc += 1;
            },
            0x52 => {
                my_log!(self,"ld d, d");
                self.d = self.d;
                self.cycles += 4;
                pc += 1;
            },
            0x53 => {
                my_log!(self,"ld d, e");
                self.d = self.e;
                self.cycles += 4;
                pc += 1;
            },
            0x54 => {
                my_log!(self,"ld d, h");
                self.d = self.h;
                self.cycles += 4;
                pc += 1;
            },
            0x55 => {
                my_log!(self,"ld d, l");
                self.d = self.l;
                self.cycles += 4;
                pc += 1;
            },
            0x56 => {
                my_log!(self,"ld d, (hl)");
                self.d = mm.read(self.hl());
                self.cycles += 8;
                pc += 1;
            },
            0x57 => {
                my_log!(self,"ld d, a");
                self.d = self.a;
                self.cycles += 4;
                pc += 1;
            },
            0x58 => {
                my_log!(self,"ld e, b");
                self.e = self.b;
                self.cycles += 4;
                pc += 1;
            },
            0x59 => {
                my_log!(self,"ld e, c");
                self.e = self.c;
                self.cycles += 4;
                pc += 1;
            },
            0x5a => {
                my_log!(self,"ld e, d");
                self.e = self.d;
                self.cycles += 4;
                pc += 1;
            },
            0x5b => {
                my_log!(self,"ld e, e");
                self.e = self.e;
                self.cycles += 4;
                pc += 1;
            },
            0x5c => {
                my_log!(self,"ld e, h");
                self.e = self.h;
                self.cycles += 4;
                pc += 1;
            },
            0x5d => {
                my_log!(self,"ld e, l");
                self.e = self.l;
                self.cycles += 4;
                pc += 1;
            },
            0x5e => {
                my_log!(self,"ld e, (hl)");
                self.e = mm.read(self.hl());
                self.cycles += 8;
                pc += 1;
            },
            0x5f => {
                my_log!(self,"ld e, a");
                self.e = self.a;
                self.cycles += 4;
                pc += 1;
            },
            0x60 => {
                my_log!(self,"ld h, b");
                self.h = self.b;
                self.cycles += 4;
                pc += 1;
            },
            0x61 => {
                my_log!(self,"ld h, c");
                self.h = self.c;
                self.cycles += 4;
                pc += 1;
            },
            0x62 => {
                my_log!(self,"ld h, d");
                self.h = self.d;
                self.cycles += 4;
                pc += 1;
            },
            0x63 => {
                my_log!(self,"ld h, e");
                self.h = self.e;
                self.cycles += 4;
                pc += 1;
            },
            0x64 => {
                my_log!(self,"ld h, h");
                self.h = self.h;
                self.cycles += 4;
                pc += 1;
            },
            0x65 => {
                my_log!(self,"ld h, l");
                self.h = self.l;
                self.cycles += 4;
                pc += 1;
            },
            0x66 => {
                my_log!(self,"ld h, (hl)");
                self.h = mm.read(self.hl());
                self.cycles += 8;
                pc += 1;
            },
            0x67 => {
                my_log!(self,"ld h, a");
                self.h = self.a;
                self.cycles += 4;
                pc += 1;
            },
            0x68 => {
                my_log!(self,"ld l, b");
                self.l = self.b;
                self.cycles += 4;
                pc += 1;
            },
            0x69 => {
                my_log!(self,"ld l, c");
                self.l = self.c;
                self.cycles += 4;
                pc += 1;
            },
            0x6a => {
                my_log!(self,"ld l, d");
                self.l = self.d;
                self.cycles += 4;
                pc += 1;
            },
            0x6b => {
                my_log!(self,"ld l, e");
                self.l = self.e;
                self.cycles += 4;
                pc += 1;
            },
            0x6c => {
                my_log!(self,"ld l, h");
                self.l = self.h;
                self.cycles += 4;
                pc += 1;
            },
            0x6d => {
                my_log!(self,"ld l, l");
                self.l = self.l;
                self.cycles += 4;
                pc += 1;
            },
            0x6e => {
                my_log!(self,"ld l, (hl)");
                self.l = mm.read(self.hl());
                self.cycles += 8;
                pc += 1;
            },
            0x6f => {
                my_log!(self,"ld l, a");
                self.l = self.a;
                self.cycles += 4;
                pc += 1;
            },
            0x70 => {
                my_log!(self,"ld (hl), b");
                mm.write(self.hl(), self.b);
                self.cycles += 8;
                pc += 1;
            },
            0x71 => {
                my_log!(self,"ld (hl), c");
                mm.write(self.hl(), self.c);
                self.cycles += 8;
                pc += 1;
            },
            0x72 => {
                my_log!(self,"ld (hl), d");
                mm.write(self.hl(), self.d);
                self.cycles += 8;
                pc += 1;
            },
            0x73 => {
                my_log!(self,"ld (hl), e");
                mm.write(self.hl(), self.e);
                self.cycles += 8;
                pc += 1;
            },
            0x74 => {
                my_log!(self,"ld (hl), h");
                mm.write(self.hl(), self.h);
                self.cycles += 8;
                pc += 1;
            },
            0x75 => {
                my_log!(self,"ld (hl), l");
                mm.write(self.hl(), self.l);
                self.cycles += 8;
                pc += 1;
            },
            0x76 => {
                self.halt = true;
                pc += 1;
            },
            0x77 => {
                my_log!(self,"ld (hl), a");
                mm.write(self.hl(), self.a);
                self.cycles += 8;
                pc += 1;
            },
            0x78 => {
                my_log!(self,"ld a, b");
                self.a = self.b;
                self.cycles += 4;
                pc += 1;
            },
            0x79 => {
                my_log!(self,"ld a, c");
                self.a = self.c;
                self.cycles += 4;
                pc += 1;
            },
            0x7a => {
                my_log!(self,"ld a, d");
                self.a = self.d;
                self.cycles += 4;
                pc += 1;
            },
            0x7b => {
                my_log!(self,"ld a, e");
                self.a = self.e;
                self.cycles += 4;
                pc += 1;
            },
            0x7c => {
                my_log!(self,"ld a, h");
                self.a = self.h;
                self.cycles += 4;
                pc += 1;
            },
            0x7d => {
                my_log!(self,"ld a, l");
                self.a = self.l;
                self.cycles += 4;
                pc += 1;
            },
            0x7e => {
                my_log!(self,"ld a, (hl)");
                self.a = mm.read(self.hl());
                self.cycles += 8;
                pc += 1;
            },
            0x7f => {
                my_log!(self,"ld a, a");
                self.a = self.a;
                self.cycles += 4;
                pc += 1;
            },
            0x80 => {
                my_log!(self,"add b");
                let val = self.b;
                self.add(val);
                self.cycles += 4;
                pc += 1;
            },
            0x81 => {
                my_log!(self,"add c");
                let val = self.c;
                self.add(val);
                self.cycles += 4;
                pc += 1;
            },
            0x82 => {
                my_log!(self,"add d");
                let val = self.d;
                self.add(val);
                self.cycles += 4;
                pc += 1;
            },
            0x83 => {
                my_log!(self,"add e");
                let val = self.e;
                self.add(val);
                self.cycles += 4;
                pc += 1;
            },
            0x84 => {
                my_log!(self,"add h");
                let val = self.h;
                self.add(val);
                self.cycles += 4;
                pc += 1;
            },
            0x85 => {
                my_log!(self,"add l");
                let val = self.l;
                self.add(val);
                self.cycles += 4;
                pc += 1;
            },
            0x86 => {
                my_log!(self,"add (hl)");
                let val = mm.read(self.hl());
                self.add(val);
                self.cycles += 8;
                pc += 1;
            },
            0x87 => {
                my_log!(self,"add a");
                let val = self.a;
                self.add(val);
                self.cycles += 8;
                pc += 1;
            },
            0x88 => {
                my_log!(self,"adc b");
                let val = self.b;
                self.adc(val);
                self.cycles += 4;
                pc += 1;
            },
            0x89 => {
                my_log!(self,"adc c");
                let val = self.c;
                self.adc(val);
                self.cycles += 4;
                pc += 1;
            },
            0x8a => {
                my_log!(self,"adc d");
                let val = self.d;
                self.adc(val);
                self.cycles += 4;
                pc += 1;
            },
            0x8b => {
                my_log!(self,"adc e");
                let val = self.e;
                self.adc(val);
                self.cycles += 4;
                pc += 1;
            },
            0x8c => {
                my_log!(self,"adc h");
                let val = self.h;
                self.adc(val);
                self.cycles += 4;
                pc += 1;
            },
            0x8d => {
                my_log!(self,"adc l");
                let val = self.l;
                self.adc(val);
                self.cycles += 4;
                pc += 1;
            },
            0x8e => {
                my_log!(self,"adc (hl)");
                let val = mm.read(self.hl());;
                self.adc(val);
                self.cycles += 8;
                pc += 1;
            },
            0x8f => {
                my_log!(self,"adc a");
                let val = self.a;
                self.adc(val);
                self.cycles += 4;
                pc += 1;
            },
            0x90 => {
                my_log!(self,"sub b");
                let val = self.b;
                self.sub(val);
                self.cycles += 4;
                pc += 1;
            },
            0x91 => {
                my_log!(self,"sub c");
                let val = self.c;
                self.sub(val);
                self.cycles += 4;
                pc += 1;
            },
            0x92 => {
                my_log!(self,"sub d");
                let val = self.d;
                self.sub(val);
                self.cycles += 4;
                pc += 1;
            },
            0x93 => {
                my_log!(self,"sub e");
                let val = self.e;
                self.sub(val);
                self.cycles += 4;
                pc += 1;
            },
            0x94 => {
                my_log!(self,"sub h");
                let val = self.h;
                self.sub(val);
                self.cycles += 4;
                pc += 1;
            },
            0x95 => {
                my_log!(self,"sub l");
                let val = self.l;
                self.sub(val);
                self.cycles += 4;
                pc += 1;
            },
            0x96 => {
                my_log!(self,"sub (hl)");
                let val = mm.read(self.hl());
                self.sub(val);
                self.cycles += 8;
                pc += 1;
            },
            0x97 => {
                my_log!(self,"sub a");
                let val = self.a;
                self.sub(val);
                self.cycles += 4;
                pc += 1;
            },
            0x98 => {
                my_log!(self,"sbc b");
                let val = self.b;
                self.sbc(val);
                self.cycles += 4;
                pc += 1;
            },
            0x99 => {
                my_log!(self,"sbc c");
                let val = self.c;
                self.sbc(val);
                self.cycles += 4;
                pc += 1;
            },
            0x9a => {
                my_log!(self,"sbc d");
                let val = self.d;
                self.sbc(val);
                self.cycles += 4;
                pc += 1;
            },
            0x9b => {
                my_log!(self,"sbc e");
                let val = self.e;
                self.sbc(val);
                self.cycles += 4;
                pc += 1;
            },
            0x9c => {
                my_log!(self,"sbc h");
                let val = self.h;
                self.sbc(val);
                self.cycles += 4;
                pc += 1;
            },
            0x9d => {
                my_log!(self,"sbc l");
                let val = self.l;
                self.sbc(val);
                self.cycles += 4;
                pc += 1;
            },
            0x9e => {
                my_log!(self,"sbc (hl)");
                let val = mm.read(self.hl());
                self.sbc(val);
                self.cycles += 8;
                pc += 1;
            },
            0x9f => {
                my_log!(self,"sbc a");
                let val = self.a;
                self.sbc(val);
                self.cycles += 4;
                pc += 1;
            },
            0xa0 => {
                my_log!(self,"and b");
                let val = self.b;
                self.and(val);
                self.cycles += 4;
                pc += 1;
            },
            0xa1 => {
                my_log!(self,"and c");
                let val = self.c;
                self.and(val);
                self.cycles += 4;
                pc += 1;
            },
            0xa2 => {
                my_log!(self,"and d");
                let val = self.d;
                self.and(val);
                self.cycles += 4;
                pc += 1;
            },
            0xa3 => {
                my_log!(self,"and e");
                let val = self.e;
                self.and(val);
                self.cycles += 4;
                pc += 1;
            },
            0xa4 => {
                my_log!(self,"and h");
                let val = self.h;
                self.and(val);
                self.cycles += 4;
                pc += 1;
            },
            0xa5 => {
                my_log!(self,"and l");
                let val = self.l;
                self.and(val);
                self.cycles += 4;
                pc += 1;
            },
            0xa6 => {
                my_log!(self,"and (hl)");
                let val = mm.read(self.hl());
                self.and(val);
                self.cycles += 8;
                pc += 1;
            },
            0xa7 => {
                my_log!(self,"and a");
                let val = self.a;
                self.and(val);
                self.cycles += 4;
                pc += 1;
            },
            0xa8 => {
                my_log!(self,"xor b");
                let val = self.b;
                self.xor(val);
                self.cycles += 4;
                pc += 1;
            },
            0xa9 => {
                my_log!(self,"xor c");
                let val = self.c;
                self.xor(val);
                self.cycles += 4;
                pc += 1;
            },
            0xaa => {
                my_log!(self,"xor d");
                let val = self.d;
                self.xor(val);
                self.cycles += 4;
                pc += 1;
            },
            0xab => {
                my_log!(self,"xor e");
                let val = self.e;
                self.xor(val);
                self.cycles += 4;
                pc += 1;
            },
            0xac => {
                my_log!(self,"xor h");
                let val = self.h;
                self.xor(val);
                self.cycles += 4;
                pc += 1;
            },
            0xad => {
                my_log!(self,"xor l");
                let val = self.l;
                self.xor(val);
                self.cycles += 4;
                pc += 1;
            },
            0xae => {
                my_log!(self,"xor (hl)");
                let val = mm.read(self.hl());
                self.xor(val);
                self.cycles += 8;
                pc += 1;
            },
            0xaf => {
                my_log!(self,"xor a");
                let val = self.a;
                self.xor(val);
                self.cycles += 4;
                pc += 1;
            },
            0xb0 => {
                my_log!(self,"or b");
                let val = self.b;
                self.or(val);
                self.cycles += 4;
                pc += 1;
            },
            0xb1 => {
                my_log!(self,"or c");
                let val = self.c;
                self.or(val);
                self.cycles += 4;
                pc += 1;
            },
            0xb2 => {
                my_log!(self,"or d");
                let val = self.d;
                self.or(val);
                self.cycles += 4;
                pc += 1;
            },
            0xb3 => {
                my_log!(self,"or e");
                let val = self.e;
                self.or(val);
                self.cycles += 4;
                pc += 1;
            },
            0xb4 => {
                my_log!(self,"or h");
                let val = self.h;
                self.or(val);
                self.cycles += 4;
                pc += 1;
            },
            0xb5 => {
                my_log!(self,"or l");
                let val = self.l;
                self.or(val);
                self.cycles += 4;
                pc += 1;
            },
            0xb6 => {
                my_log!(self,"or (hl)");
                let val = mm.read(self.hl());
                self.or(val);
                self.cycles += 8;
                pc += 1;
            },
            0xb7 => {
                my_log!(self,"or a");
                let val = self.a;
                self.or(val);
                self.cycles += 4;
                pc += 1;
            },
            0xb8 => {
                my_log!(self,"cp b");
                let val = self.b;
                self.cp(val);
                self.cycles += 4;
                pc += 1;
            },
            0xb9 => {
                my_log!(self,"cp c");
                let val = self.c;
                self.cp(val);
                self.cycles += 4;
                pc += 1;
            },
            0xba => {
                my_log!(self,"cp d");
                let val = self.d;
                self.cp(val);
                self.cycles += 4;
                pc += 1;
            },
            0xbb => {
                my_log!(self,"cp e");
                let val = self.e;
                self.cp(val);
                self.cycles += 4;
                pc += 1;
            },
            0xbc => {
                my_log!(self,"cp h");
                let val = self.h;
                self.cp(val);
                self.cycles += 4;
                pc += 1;
            },
            0xbd => {
                my_log!(self,"cp l");
                let val = self.l;
                self.cp(val);
                self.cycles += 4;
                pc += 1;
            },
            0xbe => {
                my_log!(self,"cp (hl)");
                let val = mm.read(self.hl());
                self.cp(val);
                self.cycles += 8;
                pc += 1;
            },
            0xbf => {
                my_log!(self,"cp a");
                let val = self.a;
                self.cp(val);
                self.cycles += 4;
                pc += 1;
            },
            0xc0 => {
                my_log!(self,"ret nz");
                if !self.zero() {
                    let addr = self.stack_read_u16(mm);
                    self.cycles += 20;
                    pc = addr;
                } else {
                    self.cycles += 8;
                    pc += 1;
                }
            },
            0xc1 => {
                my_log!(self,"pop bc");
                let val = self.stack_read_u16(mm);
                self.set_bc(val);
                self.cycles += 12;
                pc += 1;
            },
            0xc2 => {
                let val = self.read_u16(mm, pc + 1);
                my_log!(self,"jp nz, ${:04x}", val);
                if !self.zero() {
                    self.cycles += 16;
                    pc = val;
                } else {
                    self.cycles += 12;
                    pc += 3;
                }
            },
            0xc3 => {
                let val = self.read_u16(mm, pc + 1);
                my_log!(self,"jp ${:04x}", val);
                self.cycles += 16;
                pc = val;
            },
            0xc4 => {
                let val = self.read_u16(mm, pc + 1);
                my_log!(self,"call nz, ${:04x}", val);
                if !self.zero() {
                    let addr = self.pc + 3;
                    self.stack_write_u16(mm, addr);
                    self.cycles += 24;
                    pc = val;
                } else {
                    self.cycles += 12;
                    pc += 3;
                }
            },
            0xc5 => {
                my_log!(self,"push bc");
                let val = self.bc();
                self.stack_write_u16(mm, val);
                self.cycles += 16;
                pc += 1;
            },
            0xc6 => {
                let val = mm.read(pc + 1);
                my_log!(self,"add a, ${:02x}", val);
                self.add(val);
                self.cycles += 8;
                pc += 2;
            },
            0xc7 => {
                my_log!(self,"rst 00");
                let addr = self.pc + 1;
                self.stack_write_u16(mm, addr);
                self.cycles += 16;
                pc = 0x0;
            },
            0xc8 => {
                my_log!(self,"ret z");
                if self.zero() {
                    let addr = self.stack_read_u16(mm);
                    self.cycles += 20;
                    pc = addr;
                } else {
                    self.cycles += 8;
                    pc += 1;
                }
            },
            0xc9 => {
                my_log!(self,"ret");
                let addr = self.stack_read_u16(mm);
                self.cycles += 16;
                pc = addr;
            },
            0xca => {
                let val = self.read_u16(mm, pc + 1);
                my_log!(self,"jp z, ${:04x}", val);
                if self.zero() {
                    self.cycles += 16;
                    pc = val;
                } else {
                    self.cycles += 12;
                    pc += 3;
                }
            },
            0xcb => {
                //my_log!(self,"prefix cb");
                let c = self.handle_cb(mm);
                self.cycles += c;
                pc += 2;
            },
            0xcc => {
                let val = self.read_u16(mm, pc + 1);
                my_log!(self,"call z, ${:04x}", val);
                if self.zero() {
                    let addr = self.pc + 3;
                    self.stack_write_u16(mm, addr);
                    self.cycles += 24;
                    pc = val;
                } else {
                    self.cycles += 12;
                    pc += 3;
                }
            },
            0xcd => {
                let val = self.read_u16(mm, pc + 1);
                my_log!(self,"call ${:04x}", val);
                let addr = self.pc + 3;
                self.stack_write_u16(mm, addr);
                self.cycles += 24;
                pc = val;
            },
            0xce => {
                let val = mm.read(pc + 1);
                my_log!(self,"adc ${:02x}", val);
                self.adc(val);
                self.cycles += 8;
                pc += 2;
            },
            0xcf => {
                my_log!(self,"rst 08");
                let addr = self.pc + 1;
                self.stack_write_u16(mm, addr);
                self.cycles += 16;
                pc = 0x8;
            },
            0xd0 => {
                my_log!(self,"ret nc");
                if !self.carry() {
                    let addr = self.stack_read_u16(mm);
                    self.cycles += 20;
                    pc = addr;
                } else {
                    self.cycles += 8;
                    pc += 1;
                }
            },
            0xd1 => {
                my_log!(self,"pop de");
                let val = self.stack_read_u16(mm);
                self.set_de(val);
                self.cycles += 12;
                pc += 1;
            },
            0xd2 => {
                let val = self.read_u16(mm, pc + 1);
                my_log!(self,"jp nc, ${:04x}", val);
                if !self.carry() {
                    self.cycles += 16;
                    pc = val;
                } else {
                    self.cycles += 12;
                    pc += 3;
                }
            },
            0xd4 => {
                let val = self.read_u16(mm, pc + 1);
                my_log!(self,"call nc, ${:04x}", val);
                if !self.carry() {
                    let addr = self.pc + 3;
                    self.stack_write_u16(mm, addr);
                    self.cycles += 24;
                    pc = val;
                } else {
                    self.cycles += 12;
                    pc += 3;
                }
            },
            0xd5 => {
                my_log!(self,"push de");
                let val = self.de();
                self.stack_write_u16(mm, val);
                self.cycles += 16;
                pc += 1;
            },
            0xd6 => {
                let val = mm.read(pc + 1);
                my_log!(self,"sub ${:02x}", val);
                self.sub(val);
                self.cycles += 8;
                pc += 2;
            },
            0xd7 => {
                my_log!(self,"rst 10");
                let addr = self.pc + 1;
                self.stack_write_u16(mm, addr);
                self.cycles += 16;
                pc = 0x10;
            },
            0xd8 => {
                my_log!(self,"ret c");
                if self.carry() {
                    let addr = self.stack_read_u16(mm);
                    self.cycles += 20;
                    pc = addr;
                } else {
                    self.cycles += 8;
                    pc += 1;
                }
            },
            0xd9 => {
                my_log!(self,"reti");
                mm.interrupt_master_enable = true;
                let addr = self.stack_read_u16(mm);
                self.cycles += 16;
                pc = addr;
            },
            0xda => {
                let val = self.read_u16(mm, pc + 1);
                my_log!(self,"jp c, ${:04x}", val);
                if self.carry() {
                    self.cycles += 16;
                    pc = val;
                } else {
                    self.cycles += 12;
                    pc += 3;
                }
            },
            0xdc => {
                let val = self.read_u16(mm, pc + 1);
                my_log!(self,"call c, ${:04x}", val);
                if self.carry() {
                    let addr = self.pc + 3;
                    self.stack_write_u16(mm, addr);
                    self.cycles += 24;
                    pc = val;
                } else {
                    self.cycles += 12;
                    pc += 3;
                }
            },
            0xde => {
                let val = mm.read(pc + 1);
                my_log!(self,"sbc ${:02x}", val);
                self.sbc(val);
                self.cycles += 8;
                pc += 2;
            },
            0xdf => {
                my_log!(self,"rst 18");
                let addr = self.pc + 1;
                self.stack_write_u16(mm, addr);
                self.cycles += 16;
                pc = 0x18;
            },
            0xe0 => {
                let val = mm.read(pc + 1);
                my_log!(self,"ld ($ff00+{:02x}), a '{}'", val, self.a as char);
                let addr = 0xff00 + val as u16;
                mm.write(addr, self.a);
                self.cycles += 12;
                pc += 2;
            },
            0xe1 => {
                my_log!(self,"pop hl");
                let val = self.stack_read_u16(mm);
                self.set_hl(val);
                self.cycles += 12;
                pc += 1;
            },
            0xe2 => {
                my_log!(self,"ld ($ff00+c), a");
                let addr = 0xff00 + self.c as u16;
                mm.write(addr, self.a);
                self.cycles += 8;
                pc += 1;
            },
            0xe5 => {
                my_log!(self,"push hl");
                let val = self.hl();
                self.stack_write_u16(mm, val);
                self.cycles += 16;
                pc += 1;
            },
            0xe6 => {
                let val = mm.read(pc + 1);
                my_log!(self,"and ${:02x}", val);
                self.and(val);
                self.cycles += 8;
                pc += 2;
            },
            0xe7 => {
                my_log!(self,"rst $20");
                let addr = self.pc + 1;
                self.stack_write_u16(mm, addr);
                self.cycles += 16;
                pc = 0x20;
            },
            0xe8 => {
                let val = mm.read(pc + 1);
                my_log!(self,"add sp, {}", val as i8);
                let sp = self.sp;
                self.sp = self.sp.wrapping_add(val as i8 as u16);
                self.set_zero(false);
                self.set_subtract(false);
                self.set_half_carry((sp & 0xf) + (val as i8 as u16 & 0xf) > 0xf);
                self.set_carry((sp & 0xff) + (val as i8 as u16 & 0xff) > 0xff);
                self.cycles += 16;
                pc += 2;
            },
            0xe9 => {
                my_log!(self,"jp hl");
                self.cycles += 4;
                pc = self.hl();
            },
            0xea => {
                let val = self.read_u16(mm, pc + 1);
                my_log!(self,"ld (${:04x}), a", val);
                let a = self.a;
                mm.write(val, a);
                self.cycles += 16;
                pc += 3;
            },
            0xee => {
                let val = mm.read(pc + 1);
                my_log!(self,"xor ${:02x}", val);
                self.xor(val);
                self.cycles += 8;
                pc += 2;
            },
            0xef => {
                my_log!(self,"rst $28");
                let addr = self.pc + 1;
                self.stack_write_u16(mm, addr);
                self.cycles += 16;
                pc = 0x28;
            },
            0xf0 => {
                let val = mm.read(pc + 1);
                my_log!(self,"ld a, ($ff00+{:02x})", val);
                let addr = 0xff00 + val as u16;
                self.a = mm.read(addr);
                self.cycles += 12;
                pc += 2;
            },
            0xf1 => {
                my_log!(self,"pop af");
                let val = self.stack_read_u16(mm);
                self.set_af(val);
                self.cycles += 12;
                pc += 1;
            },
            0xf2 => {
                my_log!(self,"ld a, ($ff00+c)");
                let addr = 0xff00 + self.c as u16;
                self.a = mm.read(addr);
                self.cycles += 8;
                pc += 1;
            },
            0xf3 => {
                my_log!(self,"di");
                mm.di();
                self.cycles += 4;
                pc += 1;
            },
            0xf5 => {
                my_log!(self,"push af");
                let val = self.af();
                self.stack_write_u16(mm, val);
                self.cycles += 16;
                pc += 1;
            },
            0xf6 => {
                let val = mm.read(pc + 1);
                my_log!(self,"or ${:02x}", val);
                self.or(val);
                self.cycles += 8;
                pc += 2;
            },
            0xf7 => {
                my_log!(self,"rst $30");
                let addr = self.pc + 1;
                self.stack_write_u16(mm, addr);
                self.cycles += 16;
                pc = 0x30;
            },
            0xf8 => {
                let val = mm.read(pc + 1);
                my_log!(self,"ld hl, sp+{}", val as i8);
                let sp = self.sp;
                self.set_hl(sp.wrapping_add(val as i8 as u16));
                self.set_zero(false);
                self.set_subtract(false);
                self.set_half_carry((sp & 0xf) + (val as i8 as u16 & 0xf) > 0xf);
                self.set_carry((sp & 0xff) + (val as i8 as u16 & 0xff) > 0xff);
                self.cycles += 12;
                pc += 2;
            },
            0xf9 => {
                trace!("ld sp, hl");
                self.sp = self.hl();
                self.cycles += 8;
                pc += 1;
            },
            0xfa => {
                let addr = self.read_u16(mm, pc + 1);
                my_log!(self,"ld a, (${:04x})", addr);
                let val = mm.read(addr);
                self.a = val;
                self.cycles += 16;
                pc += 3;
            },
            0xfb => {
                my_log!(self,"ei");
                mm.ei();
                self.cycles += 4;
                pc += 1;
            },
            0xfe => {
                let val = mm.read(pc + 1);
                my_log!(self,"cp ${:02x}", val);
                self.cp(val);
                self.cycles += 8;
                pc += 2;
            },
            0xff => {
                my_log!(self,"rst $38");
                let addr = self.pc + 1;
                self.stack_write_u16(mm, addr);
                self.cycles += 16;
                pc = 0x38;
            },
            _ => panic!("unknown instruction {:02x} @ pc={:04x}", mm.read(pc), pc),
        }

        self.pc = pc;
        self.service_interrupts(mm);
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

    cpu.stack_write_u16(&mut mm, 0x1234);
    assert_eq!(cpu.sp, 0xfffc);
    assert_eq!(mm.read(cpu.sp), 0x34);
    assert_eq!(mm.read(cpu.sp + 1), 0x12);
    assert_eq!(cpu.stack_read_u16(&mut mm), 0x1234);

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
