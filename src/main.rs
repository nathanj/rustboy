#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

#[macro_use] extern crate log;
extern crate env_logger;
extern crate sdl2;

use std::io::prelude::*;
use std::fs::File;
use std::env;
use std::fmt;
use std::cell::RefCell;
use std::rc::Rc;

use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::Texture;

mod cpu;
mod lcd;
mod timer;
mod interrupt;
mod mem;

pub struct Gameboy {
    pub cpu: cpu::Cpu,
    pub mm: mem::MemoryMap,
    pub lcd : Rc<RefCell<lcd::Lcd>>,
    //vram: [u8; 0x2000],
    //eram: [u8; 0x2000],
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

    let cpu = cpu::Cpu::new();
    let vram : [u8; 0x2000] = [0; 0x2000];
    let wram : [u8; 0x2000] = [0; 0x2000];
    let hram : [u8; 0x80] = [0; 0x80];
    let iobuf : [u8; 0x100] = [0; 0x100];
    let lcd = Rc::new(RefCell::new(lcd::Lcd::new()));
    let mm = mem::MemoryMap { rom: rom, vram: vram, wram: wram, hram: hram,
        iobuf: iobuf,
        interrupt_enable: 0, interrupt_master_enable: false, interrupt_flag: 0,
        oam: [0; 40],
        lcd: lcd.clone() };
    let mut gb = Gameboy { cpu: cpu, mm: mm, lcd: lcd.clone() };

    println!("cpu = {:?}", gb.cpu);
    let mut prevcycles = 0u32;
    loop {
        let cycles = gb.cpu.run(&mut gb.mm);
        gb.lcd.borrow_mut().run(&mut gb.mm, cycles - prevcycles);
        prevcycles = cycles;
        if cycles > 1000000 {
            break;
        }
    }

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

    texture.update(None, &pixels, pitch).unwrap();

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
        break 'running
        // The rest of the game loop goes here...
    }
}
