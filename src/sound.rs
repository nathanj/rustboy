use std::fmt;
use mem;
use interrupt;

pub struct Sound {
    // channel 1 - tone and sweep
    pub nr10 : u8, // sweep register (r/w)
    pub nr11 : u8, // sound length / wave pattern duty (r/w)
    pub nr12 : u8, // volume envelope (r/w)
    pub nr13 : u8, // frequency low (w)
    pub nr14 : u8, // frequency high (r/w)

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
        println!("{:?}", self);
    }

}
