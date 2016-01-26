#[macro_use] extern crate log;
extern crate env_logger;
extern crate sdl2;

use std::io::prelude::*;
use std::fs::File;
use std::env;
use std::fmt;

use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::Texture;

struct Gameboy {
    cpu: Cpu,
    vram: [u8; 0x2000],
    eram: [u8; 0x2000],
}

struct Cpu {
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

impl Cpu {
    fn new() -> Cpu {
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

    fn read_u16(&self, rom: &Vec<u8>, pos: usize) -> u16 {
        return (rom[pos + 1] as u16) << 8 | (rom[pos] as u16);
    }

    fn run(&mut self, rom: &Vec<u8>) {
        let mut pc = self.pc as usize;
        match rom[pc] {
            0x00 => {
                trace!("nop");
                self.cycles += 4;
                pc += 1;
            },
            0x01 => {
                let val = self.read_u16(&rom, pc + 1);
                trace!("ld bc, ${}", val);
                self.set_bc(val);
                self.cycles += 12;
                pc += 3;
            },
            _ => panic!("unknown instruction {:02x} @ pc={:04x}", rom[pc], pc),
        }
        self.pc = pc as u16
    }
}

fn main() {
    env_logger::init().unwrap();

    let filename = env::args().nth(1).unwrap_or_else(|| panic!("must pass a rom"));
    let mut f = File::open(&filename).unwrap();
    let mut rom = Vec::new();
    let size = f.read_to_end(&mut rom).unwrap();

    println!("filename = {} size = {:?}", filename, size);

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("rust-sdl2 demo: Video", 160*3, 144*3)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    let mut renderer = window.renderer().build().unwrap();

    let mut texture = renderer.create_texture_streaming(PixelFormatEnum::RGB332, (160, 144)).unwrap();
    let mut pixels: [u8; 160*144] = [0; 160*144];

    renderer.set_draw_color(Color::RGB(255, 0, 0));
    renderer.clear();
    renderer.present();

    let pitch = 160;

    let mut cpu = Cpu::new();

    println!("cpu = {:?}", cpu);
    cpu.run(&rom);
    println!("cpu = {:?}", cpu);

    pixels[10100] = 10;
    pixels[10101] = 20;
    pixels[10102] = 30;
    pixels[10103] = 40;
    pixels[10104] = 50;
    pixels[10105] = 60;
    pixels[10106] = 70;
    pixels[10107] = 80;
    pixels[10108] = 90;
    pixels[10109] = 100;
    pixels[10110] = 110;
    pixels[10111] = 120;

    texture.update(None, &pixels, pitch);

    renderer.copy(&texture, None, None);
    renderer.present();

    let mut event_pump = sdl_context.event_pump().unwrap();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                _ => {}
            }
        }
        // The rest of the game loop goes here...
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

    let rom = vec![0x00, 0x01, 0x23, 0x45];
    assert_eq!(cpu.read_u16(&rom, 0), 0x0100);
    assert_eq!(cpu.read_u16(&rom, 2), 0x4523);
}
