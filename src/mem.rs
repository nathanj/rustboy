use lcd;

pub struct MemoryMap {
    pub rom: Vec<u8>,
    pub vram: [u8; 0x2000],
    pub wram: [u8; 0x2000],
    pub hram: [u8; 0x80],
    pub interrupt_enable : bool,
    pub interrupt_master_enable : bool,
    pub interrupt_flag : u8,
    pub lcd : lcd::Lcd,
}

impl MemoryMap {
    fn ioport_get_addr(&mut self, addr: u16) -> &mut u8 {
        match addr {
            0xff44 => &mut self.lcd.ly,
            _ => panic!("bad addr {:04x}", addr),
        }
    }

    fn mmap_get_addr(&mut self, addr: u16) -> &mut u8 {
        match addr {
            // rom bank 0
            0 ... 0x3fff => {
                return &mut self.rom[addr as usize];
            },
            // rom bank n
            0x4000 ... 0x7fff => {
                return &mut self.rom[addr as usize];
            },
            // vram
            0x8000 ... 0x9fff => {
                return &mut self.vram[addr as usize - 0x8000];
            },
            // wram
            0xc000 ... 0xe000 => {
                return &mut self.wram[addr as usize - 0xc000];
            },
            // hram
            0xff80 ... 0xfffe => {
                return &mut self.hram[addr as usize - 0xff80];
            },
            // ioports
            0xff00 ... 0xff7f => {
                return self.ioport_get_addr(addr);
            },
            _ => {
                // TODO
                panic!("mmap_get_addr() bad addr {:04x}", addr);
            }
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        let a = self.mmap_get_addr(addr);
        *a = val;
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        let a = self.mmap_get_addr(addr);
        return *a;
    }
}
