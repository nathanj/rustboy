use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::RwLock;
use std::vec::Vec;

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

    ch1_length_cycles : u32,
    ch1_volume : u8,
    ch1_envelope_cycles : u32,

    // channel 2 - tone
    pub nr21 : u8, // sound length / wave pattern duty (r/w)
    pub nr22 : u8, // volume envelope (r/w)
    pub nr23 : u8, // frequency low (w)
    pub nr24 : u8, // frequency high (r/w)

    ch2_length_cycles : u32,
    ch2_volume : u8,
    ch2_envelope_cycles : u32,

    // channel 3 - wave output
    pub nr30 : u8, // sound on/off (r/w)
    pub nr31 : u8, // sound length
    pub nr32 : u8, // select output level (r/w)
    pub nr33 : u8, // frequency lower data (w)
    pub nr34 : u8, // frequency higher data (r/w)
    pub wave_ram : [u8; 0x10],

    ch3_counter : usize,

    // channel 4 - noise
    pub nr41 : u8, // sound length (r/w)
    pub nr42 : u8, // volume envelope (r/w)
    pub nr43 : u8, // polynomial counter (r/w)
    pub nr44 : u8, // counter/consecutive; initial (r/w)

    ch4_length_cycles : u32,
    ch4_volume : u8,
    ch4_envelope_cycles : u32,

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
    pub phase3 : f32,
    pub phase4 : f32,
    pub sound : Arc<RwLock<Sound>>,
    pub samples : Vec<u8>,
}

impl AudioCallback for SoundPlayer {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        for i in 0..self.spec.samples {
            self.samples[i as usize] = 0;
        }

        {
            let s = self.sound.read().unwrap();

            if s.nr52 & 0x80 == 0 {
                return;
            }
        }

        self.handle_channel1();
        self.handle_channel2();
        self.handle_channel3();
        self.handle_channel4();

        for i in 0..self.spec.samples {
            out[i as usize] = -1.0 + self.samples[i as usize] as f32 / 45.0;
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

fn pow(a: u32, b: u32) -> u32 {
    let mut x = a;
    if b == 0 {
        return 1;
    }
    for i in 0..b {
        x *= a;
    }
    x
}

impl SoundPlayer {

    fn handle_channel1(&mut self) {
        let mut s = self.sound.write().unwrap();

        let freq_lo = s.nr13 as u32;
        let freq_hi = s.nr14 as u32 & 0b111;
        let freq = 131072 / (2048 - (freq_hi << 8 | freq_lo));
        let phase_inc = freq as f32 / self.spec.freq as f32;
        let wave_duty = s.nr11 >> 6;

        let phase_val = match wave_duty {
            0b00 => 0.125,
            0b01 => 0.250,
            0b10 => 0.500,
            0b11 => 0.750,
            _ => panic!(),
        };

        for x in self.samples.iter_mut() {
            if self.phase >= phase_val {
                *x += s.ch1_volume;
            }
            self.phase = (self.phase + phase_inc) % 1.0;
        }
    }


    fn handle_channel2(&mut self) {
        let mut s = self.sound.write().unwrap();

        let freq_lo = s.nr23 as u32;
        let freq_hi = s.nr24 as u32 & 0b111;
        let freq = 131072 / (2048 - (freq_hi << 8 | freq_lo));
        let phase_inc = freq as f32 / self.spec.freq as f32;
        let wave_duty = s.nr21 >> 6;

        let phase_val = match wave_duty {
            0b00 => 0.125,
            0b01 => 0.250,
            0b10 => 0.500,
            0b11 => 0.750,
            _ => panic!(),
        };

        for x in self.samples.iter_mut() {
            if self.phase2 >= phase_val {
                *x += s.ch2_volume;
            }
            self.phase2 = (self.phase2 + phase_inc) % 1.0;
        }
    }

    fn handle_channel3(&mut self) {
        let mut s = self.sound.write().unwrap();

        if s.nr30 & 0x80 == 0 || s.nr32 & 0b1100000 == 0 {
            return;
        }

        let freq_lo = s.nr33 as u32;
        let freq_hi = s.nr34 as u32 & 0b111;
        let freq = 65536 / (2048 - (freq_hi << 8 | freq_lo)) * 32;
        let phase_inc = freq as f32 / self.spec.freq as f32;

        let volume_divisor = match s.nr32 & 0b1100000 >> 5 {
            0 => { 1 }
            1 => { 1 }
            2 => { 2 }
            3 => { 4 }
            _ => { panic!() }
        };

        for x in self.samples.iter_mut() {
            let val = if s.ch3_counter % 2 == 0 {
                s.wave_ram[s.ch3_counter / 2] >> 4
            } else {
                s.wave_ram[s.ch3_counter / 2] & 0xf
            };
            *x += val / volume_divisor;

            self.phase3 += phase_inc;
            if self.phase3 >= 1.0 {
                self.phase3 -= 1.0;
                s.ch3_counter += 1;
                s.ch3_counter %= 32;
            }
        }
    }

    fn handle_channel4(&mut self) {
        let mut sound = self.sound.write().unwrap();

        if sound.ch4_volume == 0 {
            return;
        }

        let s = (sound.nr43 as u32 & 0xf0) >> 4;
        let mut r = (sound.nr43 as u32 & 0b111) as f32;
        if r == 0.0 { r = 0.5; }
        let mut p = pow(2, s);
        if p == 0 { p = 1; }
        let freq = 524288 as f32 / r / p as f32;
        let phase_inc = freq as f32 / self.spec.freq as f32;

        println!("ch 4 vol={}", sound.ch4_volume);

        for x in self.samples.iter_mut() {
            self.phase4 += phase_inc;
            if self.phase4 >= 1.0 {
                self.phase4 %= 1.0;
                *x += sound.ch4_volume;
            }
        }
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
            ch1_length_cycles : 0,
            ch1_volume : 0,
            ch1_envelope_cycles : 0,
            nr21 : 0,
            nr22 : 0,
            nr23 : 0,
            nr24 : 0,
            ch2_length_cycles : 0,
            ch2_volume : 0,
            ch2_envelope_cycles : 0,
            nr30 : 0,
            nr31 : 0,
            nr32 : 0,
            nr33 : 0,
            nr34 : 0,
            wave_ram : [0; 0x10],
            ch3_counter : 0,
            nr41 : 0,
            nr42 : 0,
            nr43 : 0,
            nr44 : 0,
            ch4_length_cycles : 0,
            ch4_volume : 0,
            ch4_envelope_cycles : 0,
            nr50 : 0,
            nr51 : 0,
            nr52 : 0,
        }
    }

    pub fn run(&mut self, mm: &mut mem::MemoryMap, cycles: u32) {
        //println!("{:?}", self);

        // channel 1 length
        {
            let n = (64 - (self.nr11 & 0x3f) as u32) * 16384; // 1/256 sec
            if n > 0 && (self.nr14 & 0x40) > 0 {
                self.ch1_length_cycles += cycles;
                if self.ch1_length_cycles > n {
                    //println!("ch1 handling length");
                    self.ch1_volume = 0;
                }
            }
        }

        // channel 2 length
        {
            let n = (64 - (self.nr21 & 0x3f) as u32) * 16384; // 1/256 sec
            if n > 0 && (self.nr24 & 0x40) > 0 {
                self.ch2_length_cycles += cycles;
                if self.ch2_length_cycles > n {
                    //println!("ch2 handling length");
                    self.ch2_volume = 0;
                }
            }
        }
        
        // channel 4 length
        {
            let n = (64 - (self.nr41 & 0x3f) as u32) * 16384; // 1/256 sec
            if n > 0 && (self.nr44 & 0x40) > 0 {
                self.ch4_length_cycles += cycles;
                if self.ch4_length_cycles > n {
                    //println!("ch4 handling length");
                    self.ch4_volume = 0;
                }
            }
        }

        // channel 1 envelope
        {
            let n = (self.nr12 & 0b111) as u32 * 65536; // 1/64 sec
            if n > 0 {
                self.ch1_envelope_cycles += cycles;
                if self.ch1_envelope_cycles > n {
                    self.ch1_envelope_cycles -= n;
                    //println!("handling envelope");
                    if self.nr12 & 0b1000 > 0 {
                        if self.ch1_volume < 0xf {
                            self.ch1_volume += 1;
                        }
                    } else {
                        if self.ch1_volume > 0 {
                            self.ch1_volume -= 1;
                        }
                    }
                }
            }
        }

        // channel 2 envelope
        {
            let n = (self.nr22 & 0b111) as u32 * 65536; // 1/64 sec
            if n > 0 {
                self.ch2_envelope_cycles += cycles;
                if self.ch2_envelope_cycles > n {
                    self.ch2_envelope_cycles -= n;
                    //println!("handling envelope");
                    if self.nr22 & 0b1000 > 0 {
                        if self.ch2_volume < 0xf {
                            self.ch2_volume += 1;
                        }
                    } else {
                        if self.ch2_volume > 0 {
                            self.ch2_volume -= 1;
                        }
                    }
                }
            }
        }

        // channel 4 envelope
        {
            let n = (self.nr42 & 0b111) as u32 * 65536; // 1/64 sec
            if n > 0 {
                self.ch4_envelope_cycles += cycles;
                if self.ch4_envelope_cycles > n {
                    self.ch4_envelope_cycles -= n;
                    if self.nr42 & 0b1000 > 0 {
                        if self.ch4_volume < 0xf {
                            self.ch4_volume += 1;
                        }
                    } else {
                        if self.ch4_volume > 0 {
                            self.ch4_volume -= 1;
                        }
                    }
                    //println!("ch4 handling envelope new vol={}", self.ch4_volume);
                }
            }
        }
    }

    pub fn handle_addr(&mut self, addr: u16, write: bool, val: u8) -> u8 {
        //println!("handling addr={:04x} write={} val={:02x}", addr, write, val);
        match addr {
            // chanell 1
            0xff10 => { if write { self.nr10 = val; } self.nr10 }
            0xff11 => {
                if write {
                    self.nr11 = val;
                    self.ch1_length_cycles = 0;
                }
                self.nr11
            }
            0xff12 => {
                if write {
                    self.nr12 = val;
                    self.ch1_volume = (val & 0xf0) >> 4;
                    self.ch1_envelope_cycles = 0;
                    //println!("setting ch1 volume = {:02x} {}", val, self.ch1_volume);
                }
                self.nr12
            }
            0xff13 => { if write { self.nr13 = val; } self.nr13 }
            0xff14 => { if write { self.nr14 = val; } self.nr14 }

            // channel 2
            0xff16 => {
                if write {
                    self.nr21 = val;
                    self.ch2_length_cycles = 0;
                }
                self.nr21
            }
            0xff17 => {
                if write {
                    self.nr22 = val;
                    self.ch2_volume = (val & 0xf0) >> 4;
                    self.ch2_envelope_cycles = 0;
                    //println!("setting ch2 volume = {:02x} {}", val, self.ch2_volume);
                }
                self.nr22
            }
            0xff18 => { if write { self.nr23 = val; } self.nr23 }
            0xff19 => { if write { self.nr24 = val; } self.nr24 }

            // channel 3
            0xff1a => { if write { self.nr30 = val; } self.nr30 }
            0xff1b => { if write { self.nr31 = val; } self.nr31 }
            0xff1c => { if write { self.nr32 = val; } self.nr32 }
            0xff1d => { if write { self.nr33 = val; } self.nr33 }
            0xff1e => { if write { self.nr34 = val; } self.nr34 }

            // channel 4
            0xff20 => { if write { self.nr41 = val; println!("wrote nr41={:02x}", self.nr41); } self.nr41 }
            0xff21 => {
                if write {
                    self.nr42 = val;
                    self.ch4_volume = (val & 0xf0) >> 4;
                    self.ch4_length_cycles = 0;
                    println!("wrote nr42={:02x}", self.nr42);
                }
                self.nr42
            }
            0xff22 => { if write { self.nr43 = val; println!("wrote nr43={:02x}", self.nr43); } self.nr43 }
            0xff23 => { if write { self.nr44 = val; println!("wrote nr44={:02x}", self.nr44); } self.nr44 }

            // sound control
            0xff24 => { if write { self.nr50 = val; } self.nr50 }
            0xff25 => { if write { self.nr51 = val; } self.nr51 }
            0xff26 => { if write { self.nr52 = val; } self.nr52 }

            0xff30 ... 0xff3f => { if write { self.wave_ram[addr as usize - 0xff30 as usize] = val; } self.wave_ram[addr as usize - 0xff30 as usize] }

            _ => { 0 }
        }
    }

}
