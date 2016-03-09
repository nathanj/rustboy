use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::RwLock;

use sdl2::audio::AudioCallback;
use sdl2::audio::AudioSpec;

use mem;
use interrupt;

pub struct Sound {
    // channel 1 - tone and sweep
    pub nr10 : u8, // sweep register (r/w)
    pub nr11 : u8, // sound length / wave pattern duty (r/w)
    pub nr12 : u8, // volume envelope (r/w)
    pub nr13 : u8, // frequency low (w)
    pub nr14 : u8, // frequency high (r/w)

    ch1_length_remaining : u8,

    // channel 2 - tone
    pub nr21 : u8, // sound length / wave pattern duty (r/w)
    pub nr22 : u8, // volume envelope (r/w)
    pub nr23 : u8, // frequency low (w)
    pub nr24 : u8, // frequency high (r/w)

    // channel 3 - wave output
    pub nr30 : u8, // sound on/off (r/w)
    pub nr31 : u8, // sound length
    pub nr32 : u8, // select output level (r/w)
    pub nr33 : u8, // frequency lower data (w)
    pub nr34 : u8, // frequency higher data (r/w)
    pub wave_ram : [u8; 0x10],

    // channel 4 - noise
    pub nr41 : u8, // sound length (r/w)
    pub nr42 : u8, // volume envelope (r/w)
    pub nr43 : u8, // polynomial counter (r/w)
    pub nr44 : u8, // counter/consecutive; initial (r/w)

    // sound control registers
    pub nr50 : u8, // channel control / on-off / volume (r/w)
    pub nr51 : u8, // selection of sound output terminal (r/w)
    pub nr52 : u8, // sound on/off
}


pub struct SoundPlayer {
    pub spec : AudioSpec,
    pub volume : f32,
    pub x : u8,
    pub phase : f32,
    pub phase2 : f32,
    pub sound : Arc<RwLock<Sound>>,
}

impl AudioCallback for SoundPlayer {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        let s = self.sound.read().unwrap();

        for x in out.iter_mut() {
            *x = 0.0;
        }


        /*

        // channel 3

        if s.nr30 & 0x80 == 0 || s.nr32 & 0b1100000 == 0 {
        // sound off
        for x in out.iter_mut() {
         *x = 0.0;
         }
         }

         let freq_lo = s.nr33 as u32;
         let freq_hi = s.nr34 as u32 & 0b111;
         let freq = 65536 / (2048 - (freq_hi << 8 | freq_lo));

         println!("spec = {:?}", self.spec);
         println!("s = {:?}", *s);
         println!("freq = {}", freq);
         println!("wave_ram = {:?}", s.wave_ram);

         let volume_divisor = match s.nr32 & 0b1100000 >> 5 {
         0 => { 1 }
         1 => { 1 }
         2 => { 2 }
         3 => { 4 }
         _ => { panic!() }
         };

         let mut mybuf : [f32; 32] = [0.0; 32];
         for i in 0..16 {
         mybuf[i * 2] = (s.wave_ram[i / 2] >> 4) as f32 / 16.0;
         mybuf[i * 2 + 1] = (s.wave_ram[i / 2] & 0xF) as f32 / 16.0;
         }

         let mut wave_counter = 0;
         for x in out.iter_mut() {
         *x = mybuf[wave_counter];
         wave_counter = (wave_counter + 1) % 32;
         }

*/


        // channel 1

        {
            if s.nr52 & 0x80 == 0 {
                return;
            }

            let freq_lo = s.nr13 as u32;
            let freq_hi = s.nr14 as u32 & 0b111;
            let freq = 131072 / (2048 - (freq_hi << 8 | freq_lo));
            let phase_inc = freq as f32 / self.spec.freq as f32;
            let wave_duty = s.nr11 >> 6;

            println!("spec = {:?}", self.spec);
            println!("s = {:?}", *s);
            println!("freq = {} wave_duty = {} phase_inc = {} phase = {} samples = {}",
                     freq, wave_duty, phase_inc, self.phase, self.spec.samples);

            //if s.nr10 & 0b1110000 > 0 {
            //    panic!("sweep {:?}", *s);
            //}
            //if s.nr12 & 0b111 > 0 {
            //    println!("handling envelope");
            //    if s.nr12 & 0b1000 > 0 {
            //        self.volume += 0.01;
            //    } else {
            //        self.volume -= 0.01;
            //    }
            //}

            let phase_val = match wave_duty {
                0b00 => 0.125,
                0b01 => 0.250,
                0b10 => 0.500,
                0b11 => 0.750,
                _ => panic!(),
            };

            for x in out.iter_mut() {

                *x += if self.phase >= phase_val {
                    self.volume
                } else {
                    -self.volume
                };

                self.phase = (self.phase + phase_inc) % 1.0;
            }
        }
        
        // channel 2
        {
            if s.nr52 & 0x80 == 0 {
                return;
            }

            let freq_lo = s.nr23 as u32;
            let freq_hi = s.nr24 as u32 & 0b111;
            let freq = 131072 / (2048 - (freq_hi << 8 | freq_lo));
            let phase_inc = freq as f32 / self.spec.freq as f32;
            let wave_duty = s.nr21 >> 6;

            println!("spec = {:?}", self.spec);
            println!("s = {:?}", *s);
            println!("freq = {} wave_duty = {} phase_inc = {} phase = {} samples = {}",
                     freq, wave_duty, phase_inc, self.phase, self.spec.samples);

            //if s.nr10 & 0b1110000 > 0 {
            //    panic!("sweep {:?}", *s);
            //}
            //if s.nr12 & 0b111 > 0 {
            //    println!("handling envelope");
            //    if s.nr12 & 0b1000 > 0 {
            //        self.volume += 0.01;
            //    } else {
            //        self.volume -= 0.01;
            //    }
            //}

            let phase_val = match wave_duty {
                0b00 => 0.125,
                0b01 => 0.250,
                0b10 => 0.500,
                0b11 => 0.750,
                _ => panic!(),
            };

            for x in out.iter_mut() {

                *x += if self.phase2 >= phase_val {
                    self.volume
                } else {
                    -self.volume
                };

                self.phase2 = (self.phase2 + phase_inc) % 1.0;
            }
        }
    }
}

impl fmt::Debug for Sound {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Sound {{ \n\
               nr10:{:02x} nr11:{:02x} nr12:{:02x} nr13:{:02x} nr14:{:02x} \n\
        _______ nr21:{:02x} nr22:{:02x} nr23:{:02x} nr24:{:02x} \n\
        nr30:{:02x} nr31:{:02x} nr32:{:02x} nr33:{:02x} nr34:{:02x} \n\
        _______ nr41:{:02x} nr42:{:02x} nr43:{:02x} nr44:{:02x} \n\
        nr50:{:02x} nr51:{:02x} nr52:{:02x} \
        }}",
        self.nr10,
        self.nr11,
        self.nr12,
        self.nr13,
        self.nr14,
        self.nr21,
        self.nr22,
        self.nr23,
        self.nr24,
        self.nr30,
        self.nr31,
        self.nr32,
        self.nr33,
        self.nr34,
        self.nr41,
        self.nr42,
        self.nr43,
        self.nr44,
        self.nr50,
        self.nr51,
        self.nr52)
    }
}

impl Sound {

    pub fn new() -> Sound {
        Sound {
            nr10 : 0,
            nr11 : 0,
            nr12 : 0,
            nr13 : 0,
            nr14 : 0,
            ch1_length_remaining : 0,
            nr21 : 0,
            nr22 : 0,
            nr23 : 0,
            nr24 : 0,
            nr30 : 0,
            nr31 : 0,
            nr32 : 0,
            nr33 : 0,
            nr34 : 0,
            wave_ram : [0; 0x10],
            nr41 : 0,
            nr42 : 0,
            nr43 : 0,
            nr44 : 0,
            nr50 : 0,
            nr51 : 0,
            nr52 : 0,
        }
    }

    pub fn run(&mut self, mm: &mut mem::MemoryMap) {
        //println!("{:?}", self);
    }

    pub fn handle_addr(&mut self, addr: u16, write: bool, val: u8) -> u8 {
        println!("handling addr={:04x} write={} val={:02x}", addr, write, val);
        match addr {
            0xff10 => { if write { self.nr10 = val; } self.nr10 }
            0xff11 => {
                if write {
                    self.nr11 = val;
                    self.ch1_length_remaining = val & 0x3f;
                }
                self.nr11
            }
            0xff12 => { if write { self.nr12 = val; } self.nr12 }
            0xff13 => { if write { self.nr13 = val; } self.nr13 }
            0xff14 => { if write { self.nr14 = val; } self.nr14 }


            0xff16 => { if write { self.nr21 = val; } self.nr21 }
            0xff17 => { if write { self.nr22 = val; } self.nr22 }
            0xff18 => { if write { self.nr23 = val; } self.nr23 }
            0xff19 => { if write { self.nr24 = val; } self.nr24 }
            0xff1a => { if write { self.nr30 = val; } self.nr30 }
            0xff1b => { if write { self.nr31 = val; } self.nr31 }
            0xff1c => { if write { self.nr32 = val; } self.nr32 }
            0xff1d => { if write { self.nr33 = val; } self.nr33 }
            0xff1e => { if write { self.nr34 = val; } self.nr34 }
            0xff20 => { if write { self.nr41 = val; } self.nr41 }
            0xff21 => { if write { self.nr42 = val; } self.nr42 }
            0xff22 => { if write { self.nr43 = val; } self.nr43 }
            0xff23 => { if write { self.nr44 = val; } self.nr44 }
            0xff24 => { if write { self.nr50 = val; } self.nr50 }
            0xff25 => { if write { self.nr51 = val; } self.nr51 }
            0xff26 => { if write { self.nr52 = val; } self.nr52 }

            0xff30 ... 0xff3f => { if write { self.wave_ram[addr as usize - 0xff30 as usize] = val; } self.wave_ram[addr as usize - 0xff30 as usize] }



            _ => { 0 }
        }
    }
}
