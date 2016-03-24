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
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use std::vec;
use time::Duration;

use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::Texture;
use sdl2::audio::{AudioCallback, AudioSpecDesired};

mod cpu;
mod lcd;
mod timer;
mod interrupt;
mod mem;
mod joypad;
mod sound;

struct Gameboy {
    cpu: cpu::Cpu,
    mm: mem::MemoryMap,
    lcd : Rc<RefCell<lcd::Lcd>>,
    timer : Rc<RefCell<timer::Timer>>,
    joypad : Rc<RefCell<joypad::Joypad>>,
    sound : Arc<RwLock<sound::Sound>>,
}

fn main() {
    env_logger::init().unwrap();

    let filename = env::args().nth(1).unwrap_or_else(|| panic!("must pass a rom"));
    let mut f = File::open(&filename).unwrap();
    let mut rom = Vec::new();
    let size = f.read_to_end(&mut rom).unwrap();

    println!("filename = {} size = {:?}", filename, size);

    let sdl_context = sdl2::init().unwrap();



    // Initialize the video.
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
    texture.update(None, &pixels, pitch).unwrap();
    renderer.copy(&texture, None, None);
    renderer.present();


    // Initialize the emulator.
    let cpu = cpu::Cpu::new();
    let lcd = Rc::new(RefCell::new(lcd::Lcd::new()));
    let timer = Rc::new(RefCell::new(timer::Timer::new()));
    let joypad = Rc::new(RefCell::new(joypad::Joypad::new()));
    let sound = Arc::new(RwLock::new(sound::Sound::new()));
    let mm = mem::MemoryMap {
        rom: rom,
        vram: [0; 0x2000],
        wram: [0; 0x2000],
        hram: [0; 0x80],
        eram: [0; 0x2000],
        iobuf: [0; 0x100],
        interrupt_enable: 0,
        interrupt_master_enable: false,
        interrupt_flag: 0,
        oam: [0; 0xa0],
        lcd: lcd.clone(),
        timer: timer.clone(),
        joypad: joypad.clone(),
        sound: sound.clone(),
        rom_bank: 0,
    };
    let mut gb = Gameboy {
        cpu: cpu,
        mm: mm,
        lcd: lcd.clone(),
        timer: timer.clone(),
        joypad: joypad.clone(),
        sound: sound.clone(),
    };



    // Initialize the audio.
    let audio_subsystem = sdl_context.audio().unwrap();
    let desired_spec = AudioSpecDesired {
        freq: Some(44100),
        channels: Some(1),
        samples: None,
    };
    let device = audio_subsystem.open_playback(None, desired_spec, |spec| {
        println!("spec = {:?}", spec);
        sound::SoundPlayer {
            spec: spec,
            volume: 0.05,
            x: 5,
            phase: 0.0,
            phase2: 0.0,
            phase3: 0.0,
            sound: sound.clone(),
            samples: vec![0; spec.samples as usize],
        }
    }).unwrap();
    device.resume();

    gb.mm.load_eram();


    let mut prevcycles = 0u32;
    let mut start = time::now();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut fastforward = false;
    'running: loop {
        if prevcycles % 1000000 < 10 {
            println!("cycles={}", prevcycles);
        }

        let cycles = gb.cpu.run(&mut gb.mm);
        let vblank = gb.lcd.borrow_mut().run(&mut gb.mm, cycles - prevcycles, &mut pixels);
        gb.timer.borrow_mut().run(&mut gb.mm, cycles - prevcycles);
        gb.sound.write().unwrap().run(&mut gb.mm, cycles - prevcycles);

        if vblank {
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                        gb.mm.save_eram();
                        break 'running
                    },
                    Event::KeyDown { keycode: Some(Keycode::F), .. } => {
                        fastforward = true;
                    }
                    Event::KeyUp { keycode: Some(Keycode::F), .. } => {
                        fastforward = false;
                    }
                    Event::KeyDown { keycode: Some(Keycode::D), .. } => {
                        //gb.cpu.tracing = true;
                        println!("{:?}", gb.lcd.borrow());
                        gb.mm.dump(0x8000, 0xa000 - 0x8000);
                        panic!("asdf");
                    }
                    Event::KeyUp { keycode: Some(Keycode::D), .. } => {
                        //gb.cpu.tracing = false;
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

            //gb.lcd.borrow().draw(&mut gb.mm, &mut pixels);
            texture.update(None, &pixels, pitch).unwrap();
            renderer.copy(&texture, None, None);
            renderer.present();

            let end = time::now();
            let delta = end - start;
            start = end;
            //println!("ms={}", delta.num_milliseconds());

            if !fastforward && delta.num_milliseconds() < 17 {
                thread::sleep_ms(17 - delta.num_milliseconds() as u32);
            }
        }

        prevcycles = cycles;
    }
}
