#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

#[macro_use] extern crate log;
extern crate env_logger;
extern crate sdl2;
extern crate time;

use std::io::prelude::*;
use std::fs::File;
use std::env;
use std::fmt;
use std::cell::RefCell;
use std::rc::Rc;
use std::thread;
use time::Duration;

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
mod joypad;

struct Gameboy {
    cpu: cpu::Cpu,
    mm: mem::MemoryMap,
    lcd : Rc<RefCell<lcd::Lcd>>,
    timer : Rc<RefCell<timer::Timer>>,
    joypad : Rc<RefCell<joypad::Joypad>>,
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

    let window = video_subsystem.window("rust-sdl2 demo: Video", 160*1, 144*1)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    let mut renderer = window.renderer().build().unwrap();

    let mut texture = renderer.create_texture_streaming(PixelFormatEnum::RGB332, (160, 144)).unwrap();
    let mut pixels: [u8; 160*144] = [0; 160*144];

    let pitch = 160;

    let cpu = cpu::Cpu::new();
    let vram : [u8; 0x2000] = [0; 0x2000];
    let wram : [u8; 0x2000] = [0; 0x2000];
    let hram : [u8; 0x80] = [0; 0x80];
    let iobuf : [u8; 0x100] = [0; 0x100];
    let lcd = Rc::new(RefCell::new(lcd::Lcd::new()));
    let timer = Rc::new(RefCell::new(timer::Timer::new()));
    let joypad = Rc::new(RefCell::new(joypad::Joypad::new()));
    let mm = mem::MemoryMap { rom: rom, vram: vram, wram: wram, hram: hram,
        iobuf: iobuf,
        interrupt_enable: 0, interrupt_master_enable: false, interrupt_flag: 0,
        oam: [0; 0xa0],
        lcd: lcd.clone(),
        timer: timer.clone(),
        joypad: joypad.clone(),
    };
    let mut gb = Gameboy {
        cpu: cpu,
        mm: mm,
        lcd: lcd.clone(),
        timer: timer.clone(),
        joypad: joypad.clone(),
    };

    texture.update(None, &pixels, pitch).unwrap();
    renderer.copy(&texture, None, None);
    renderer.present();

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut prevcycles = 0u32;
    let mut drawcycles = 0u32;
    let mut start = time::now();
    'running: loop {
        if prevcycles % 1000000 < 10 {
            println!("cycles={}", prevcycles);
        }

        // The rest of the game loop goes here...
        let cycles = gb.cpu.run(&mut gb.mm);
        gb.lcd.borrow_mut().run(&mut gb.mm, cycles - prevcycles);
        gb.timer.borrow_mut().run(&mut gb.mm, cycles - prevcycles);

        drawcycles += cycles - prevcycles;
        if drawcycles > 70224 {
            drawcycles -= 70224;

            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                        break 'running
                    },
                    Event::KeyDown { keycode: Some(Keycode::D), .. } => {
                        gb.cpu.tracing = true;
                    }
                    Event::KeyUp { keycode: Some(Keycode::D), .. } => {
                        gb.cpu.tracing = false;
                    }
                    Event::KeyDown { keycode: Some(keycode), .. } => {
                        joypad.borrow_mut().handle_input(&mut gb.mm, keycode, true);
                    }
                    Event::KeyUp { keycode: Some(keycode), .. } => {
                        joypad.borrow_mut().handle_input(&mut gb.mm, keycode, false);
                    }
                    _ => {}
                }
            }

            gb.lcd.borrow().draw(&mut gb.mm, &mut pixels);
            texture.update(None, &pixels, pitch).unwrap();
            renderer.copy(&texture, None, None);
            renderer.present();

            let end = time::now();
            let delta = end - start;
            start = end;
            //println!("ms={}", delta.num_milliseconds());

            //if delta.num_milliseconds() < 16 {
            //    thread::sleep_ms(16 - delta.num_milliseconds() as u32);
            //}

            //break 'running;
        }

        prevcycles = cycles;
    }
}
