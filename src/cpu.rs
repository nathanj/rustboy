use std::fmt;

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

fn mmap_write(addr: u16, val: u8) {
    // TODO
}

fn mmap_read(addr: u16) -> u8 {
    // TODO
    return 0;
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

    fn read_u16(&self, rom: &Vec<u8>, pos: usize) -> u16 {
        return (rom[pos + 1] as u16) << 8 | (rom[pos] as u16);
    }

    pub fn run(&mut self, rom: &Vec<u8>) {
        let mut pc = self.pc as usize;
        match rom[pc] {
            0x00 => {
                trace!("nop");
                self.cycles += 4;
                pc += 1;
            },
            0x01 => {
                let val = self.read_u16(&rom, pc + 1);
                trace!("ld bc, ${:04x} (#{})", val, val);
                self.set_bc(val);
                self.cycles += 12;
                pc += 3;
            },
            0x02 => {
                trace!("ld (bc), a");
                mmap_write(self.bc(), self.a);
                self.cycles += 8;
                pc += 1;
            },
            0x03 => {
                trace!("inc bc");
                let bc = self.bc();
                self.set_bc(bc + 1);
                self.cycles += 8;
                pc += 1;
            },
            0x04 => {
                trace!("inc b");
                self.b += 1;
                let b = self.b;
                self.set_zero(b == 0);
                self.set_subtract(false);
                self.set_half_carry(b & 0xf == 0);
                self.cycles += 4;
                pc += 1;
            },
            0x05 => {
                trace!("dec b");
                self.b -= 1;
                let b = self.b;
                self.set_zero(b == 0);
                self.set_subtract(true);
                self.set_half_carry(b & 0xf == 0xf);
                self.cycles += 4;
                pc += 1;
            },
            0x06 => {
                let val = rom[pc + 1];
                trace!("ld b, ${:02x} (#{})", val, val);
                self.b = val;
                self.cycles += 8;
                pc += 2;
            },
            0x07 => {
                trace!("rlca");
                // TODO
                self.cycles += 4;
                pc += 1;
                self.set_zero(false);
                self.set_subtract(false);
                self.set_half_carry(false);
                self.set_carry(true /* TODO */);
            },
            0x08 => {
                let val = self.read_u16(&rom, pc + 1);
                trace!("ld (${:04x}), sp", val);
                self.sp = val;
                self.cycles += 20;
                pc += 3;
            },
            0x09 => {
                trace!("add hl, bc");
                let bc = self.bc();
                let hl = self.hl();
                self.set_hl(hl + bc);
                self.set_subtract(false);
                self.set_half_carry(false);
                self.set_carry(false /* TODO */);
                self.cycles += 8;
                pc += 1;
            },
            0x0a => {
                trace!("ld a, (bc)");
                self.a = mmap_read(self.bc());
                self.cycles += 8;
                pc += 1;
            },
            0x0b => {
                trace!("dec bc");
                let bc = self.bc();
                self.set_bc(bc - 1);
                self.cycles += 8;
                pc += 1;
            },
            0x0c => {
                trace!("inc c");
                self.c += 1;
                self.cycles += 4;
                pc += 1;
            },
            0x0d => {
                trace!("dec c");
                self.c -= 1;
                self.cycles += 4;
                pc += 1;
            },
            0x0e => {
                let val = rom[pc + 1];
                trace!("ld c, ${:02x}", val);
                self.c = val;
                self.cycles += 8;
                pc += 2;
            },
            0x0f => {
                trace!("rrca");
                // TODO
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
                let val = self.read_u16(&rom, pc + 1);
                trace!("ld de, ${:04x}", val);
                self.set_de(val);
                self.cycles += 12;
                pc += 3;
            },
            0x12 => {
                trace!("ld (de), a");
                mmap_write(self.de(), self.a);
                self.cycles += 8;
                pc += 1;
            },
            0x13 => {
                trace!("inc de");
                let de = self.de();
                self.set_de(de + 1);
                self.cycles += 8;
                pc += 1;
            },
            0x14 => {
                trace!("inc d");
                self.d += 1;
                let d = self.d;
                self.set_zero(d == 0);
                self.set_subtract(false);
                self.set_half_carry(d & 0xf == 0);
                self.cycles += 4;
                pc += 1;
            },
            0x15 => {
                trace!("dec d");
                self.d -= 1;
                let d = self.d;
                self.set_zero(d == 0);
                self.set_subtract(true);
                self.set_half_carry(d & 0xf == 0xf);
                self.cycles += 4;
                pc += 1;
            },
            0x16 => {
                let val = rom[pc + 1];
                trace!("ld d, ${:02x} (#{})", val, val);
                self.d = val;
                self.cycles += 8;
                pc += 2;
            },
            0x17 => {
                trace!("rla");
                // TODO
                self.cycles += 4;
                pc += 1;
            },
            0x18 => {
                let val = rom[pc + 1];
                trace!("jr #{:02x}", val);
                pc += val as usize;
                self.cycles += 12;
                pc += 2;
            },
            0x19 => {
                trace!("add hl, de");
                let de = self.de();
                let hl = self.hl();
                self.set_hl(hl + de);
                self.set_subtract(false);
                self.set_half_carry(false);
                self.set_carry(false /* TODO */);
                self.cycles += 8;
                pc += 1;
            },
            0x1a => {
                trace!("ld a, (de)");
                self.a = mmap_read(self.de());
                self.cycles += 8;
                pc += 1;
            },
            0x1b => {
                trace!("dec de");
                let de = self.de();
                self.set_de(de - 1);
                self.cycles += 8;
                pc += 1;
            },
            0x1c => {
                trace!("inc e");
                self.e += 1;
                self.cycles += 4;
                pc += 1;
            },
            0x1d => {
                trace!("dec e");
                self.e -= 1;
                self.cycles += 4;
                pc += 1;
            },
            0x1e => {
                let val = rom[pc + 1];
                trace!("ld e, ${:02x}", val);
                self.e = val;
                self.cycles += 8;
                pc += 2;
            },
            0x1f => {
                trace!("rra");
                // TODO
                self.cycles += 4;
                pc += 1;
            },
            0x20 => {
                let val = rom[pc + 1];
                trace!("jr nz, #{:02x}", val);
                if !self.zero() {
                    pc += val as usize;
                    self.cycles += 12;
                } else {
                    self.cycles += 8;
                }
                pc += 2;
            },
            0x21 => {
                let val = self.read_u16(&rom, pc + 1);
                trace!("ld hl, ${:04x}", val);
                self.set_hl(val);
                self.cycles += 12;
                pc += 3;
            },
            0x22 => {
                trace!("ld (hl+), a");
                let hl = self.hl();
                mmap_write(hl, self.a);
                self.set_hl(hl + 1);
                self.cycles += 8;
                pc += 1;
            },
            0x23 => {
                trace!("inc hl");
                let hl = self.hl();
                self.set_hl(hl + 1);
                self.cycles += 8;
                pc += 1;
            },
            0x24 => {
                trace!("inc h");
                self.h += 1;
                self.cycles += 4;
                pc += 1;
            },
            0x25 => {
                trace!("dec h");
                self.h -= 1;
                self.cycles += 4;
                pc += 1;
            },
            0x26 => {
                let val = rom[pc + 1];
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
                let val = rom[pc + 1];
                trace!("jr z, ${:02x}", val);
                if self.zero() {
                    pc += val as usize;
                    self.cycles += 12;
                } else {
                    self.cycles += 8;
                }
                pc += 2;
            },
            0x29 => {
                trace!("add hl, hl");
                let hl = self.hl();
                self.set_hl(hl + hl);
                self.cycles += 8;
                pc += 1;
            },
            0x2a => {
                trace!("ld a, (hl+)");
                let hl = self.hl();
                self.a = mmap_read(hl);
                self.set_hl(hl + 1);
                self.cycles += 8;
                pc += 1;
            },
            0x2b => {
                trace!("dec hl");
                let hl = self.hl();
                self.set_hl(hl - 1);
                self.cycles += 8;
                pc += 1;
            },
            0x2c => {
                trace!("inc l");
                self.l += 1;
                self.cycles += 4;
                pc += 1;
            },
            0x2d => {
                trace!("dec l");
                self.l -= 1;
                self.cycles += 4;
                pc += 1;
            },
            0x2e => {
                let val = rom[pc + 1];
                trace!("ld l, #{:02x}", val);
                self.l = val;
                self.cycles += 8;
                pc += 2;
            },
            0x2f => {
                trace!("cpl");
                self.a = !self.a;
                self.cycles += 4;
                pc += 1;
            },
            0x30 => {
                let val = rom[pc + 1];
                trace!("jr nc, #{:02x}", val);
                if !self.carry() {
                    pc += val as usize;
                    self.cycles += 12;
                } else {
                    self.cycles += 8;
                }
                pc += 2;
            },
            0x31 => {
                let val = self.read_u16(&rom, pc + 1);
                trace!("ld sp, ${:04x}", val);
                self.sp = val;
                self.cycles += 12;
                pc += 3;
            },
            0x32 => {
                trace!("ld (hl-), a");
                let hl = self.hl();
                mmap_write(hl, self.a);
                self.set_hl(hl - 1);
                self.cycles += 8;
                pc += 1;
            },
            0x33 => {
                trace!("inc sp");
                self.sp += 1;
                self.cycles += 8;
                pc += 1;
            },
            0x34 => {
                trace!("inc (hl)");
                let hl = self.hl();
                mmap_write(hl, mmap_read(hl) + 1);
                self.cycles += 12;
                pc += 1;
            },
            0x35 => {
                trace!("dec (hl)");
                let hl = self.hl();
                mmap_write(hl, mmap_read(hl) - 1);
                self.cycles += 12;
                pc += 1;
            },
            0x36 => {
                let val = rom[pc + 1];
                trace!("ld (hl), #{:02x}", val);
                mmap_write(self.hl(), val);
                self.cycles += 12;
                pc += 1;
            },
            0x37 => {
                trace!("scf");
                self.cycles += 4;
                pc += 1;
            },
            0x38 => {
                let val = rom[pc + 1];
                trace!("jr c, #{:02x}", val);
                if self.carry() {
                    pc += val as usize;
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
                self.set_hl(hl + sp);
                self.cycles += 8;
                pc += 2;
            },
            0x3a => {
                trace!("ld a, (hl-)");
                self.a = mmap_read(self.hl());
                let hl = self.hl();
                self.set_hl(hl - 1);
                self.cycles += 8;
                pc += 2;
            },
            0x3b => {
                trace!("dec sp");
                self.sp -= 1;
                self.cycles += 8;
                pc += 2;
            },
            0x3c => {
                trace!("inc a");
                self.a += 1;
                self.cycles += 4;
                pc += 1;
            },
            0x3d => {
                trace!("dec a");
                self.a -= 1;
                self.cycles += 4;
                pc += 1;
            },
            0x3e => {
                let val = rom[pc + 1];
                trace!("ld a, #{:02x}", val);
                self.a = val;
                self.cycles += 8;
                pc += 2;
            },
            0x3f => {
                trace!("ccf");
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
                self.b = mmap_read(self.hl());
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
                self.c = mmap_read(self.hl());
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
                self.c = mmap_read(self.hl());
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
                self.e = mmap_read(self.hl());
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
                self.h = mmap_read(self.hl());
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
                self.l = mmap_read(self.hl());
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
                mmap_write(self.hl(), self.b);
                self.cycles += 8;
                pc += 1;
            },
            0x71 => {
                trace!("ld (hl), c");
                mmap_write(self.hl(), self.c);
                self.cycles += 8;
                pc += 1;
            },
            0x72 => {
                trace!("ld (hl), d");
                mmap_write(self.hl(), self.d);
                self.cycles += 8;
                pc += 1;
            },
            0x73 => {
                trace!("ld (hl), e");
                mmap_write(self.hl(), self.e);
                self.cycles += 8;
                pc += 1;
            },
            0x74 => {
                trace!("ld (hl), h");
                mmap_write(self.hl(), self.h);
                self.cycles += 8;
                pc += 1;
            },
            0x75 => {
                trace!("ld (hl), l");
                mmap_write(self.hl(), self.l);
                self.cycles += 8;
                pc += 1;
            },
            0x76 => {
                panic!("halt");
            },
            0x77 => {
                trace!("ld (hl), a");
                mmap_write(self.hl(), self.a);
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
                self.a = mmap_read(self.hl());
                self.cycles += 8;
                pc += 1;
            },
            0x7f => {
                trace!("ld a, a");
                self.a = self.a;
                self.cycles += 4;
                pc += 1;
            },
            _ => panic!("unknown instruction {:02x} @ pc={:04x}", rom[pc], pc),
        }
        self.pc = pc as u16
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
    assert_eq!(cpu.read_u16(&rom, 0), 0x0100);
    assert_eq!(cpu.read_u16(&rom, 2), 0x4523);
}
