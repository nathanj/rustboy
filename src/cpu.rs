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
