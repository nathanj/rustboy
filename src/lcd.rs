use std::fmt;
use cpu;
use mem;
use interrupt;

#[derive(Default)]
pub struct Lcd {
	pub ctl: u8,  // LCD Control (R/W)
	pub stat: u8, // LCDC Status (R/W)
	pub scy: u8,  // Scroll Y (R/W)
	pub scx: u8,  // Scroll X (R/W)
	pub ly: u8,   // LCDC Y-Coordinate (R)
	pub lyc: u8,  // LY Compare (R/W)
	pub wy: u8,   // Window Y Position (R/W)
	pub wx: u8,   // Window X Position minus 7 (R/W)
	pub bgp: u8,  // BG Palette Data (R/W) - Non CGB Mode Only
	pub obp0: u8, // Object Palette 0 Data (R/W) - Non CGB Mode Only
	pub obp1: u8, // Object Palette 1 Data (R/W) - Non CGB Mode Only
	pub dma: u8,  // DMA Transfer and Start Address (W)
    cycles: u32,
}

const LCD_CTL_ENABLE                         : u8 = 1<<7; // (0=Off, 1=On)
const LCD_CTL_WINDOW_TILE_MAP_DISPLAY_SELECT : u8 = 1<<6; // (0=9800-9BFF, 1=9C00-9FFF)
const LCD_CTL_WINDOW_DISPLAY_ENABLE          : u8 = 1<<5; // (0=Off, 1=On)
const LCD_CTL_BG_WINDOW_TILE_DATA_SELECT     : u8 = 1<<4; // (0=8800-97FF, 1=8000-8FFF)
const LCD_CTL_BG_TILE_MAP_DISPLAY_SELECT     : u8 = 1<<3; // (0=9800-9BFF, 1=9C00-9FFF)
const LCD_CTL_OBJ_SIZE                       : u8 = 1<<2; // (0=8x8, 1=8x16)
const LCD_CTL_OBJ_DISPLAY_ENABLE             : u8 = 1<<1; // (0=Off, 1=On)
const LCD_CTL_BG_DISPLAY                     : u8 = 1<<0; // (0=Off, 1=On)

const LCD_STATUS_LY_COINCIDENCE_INTERRUPT : u8 = 1<<6;        // (1=Enable) (Read/Write)
const LCD_STATUS_MODE_2_OAM_INTERRUPT     : u8 = 1<<5;        // (1=Enable) (Read/Write)
const LCD_STATUS_MODE_1_VBLANK_INTERRUPT  : u8 = 1<<4;        // (1=Enable) (Read/Write)
const LCD_STATUS_MODE_0_HBLANK_INTERRUPT  : u8 = 1<<3;        // (1=Enable) (Read/Write)
const LCD_STATUS_COINCIDENCE              : u8 = 1<<2;        // (0:LYC<>LY, 1:LYC=LY) (Read Only)
const LCD_STATUS_MODE                     : u8 = 1<<1 | 1<<0; // (Mode 0-3) (Read Only)
                                                              //    0: During H-Blank
                                                              //    1: During V-Blank
                                                              //    2: During Searching OAM-RAM
                                                              //    3: During Transfering Data to LCD Driver

impl fmt::Debug for Lcd {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Lcd {{ \
               ctl:{:02x} stat:{:02x} scy:{:02x} scx:{:02x} ly:{:02x} \
               lyc:{:02x} wy:{:02x} wx:{:02x} bgp:{:02x} obp0:{:02x} \
               obp1:{:02x} dma:{:02x} cycles:{:04x} \
               }}",
               self.ctl, self.stat, self.scy, self.scx, self.ly, self.lyc,
               self.wy, self.wx, self.bgp, self.obp0, self.obp1, self.dma,
               self.cycles)
    }
}


impl Lcd {
    pub fn new() -> Lcd {
        let lcd: Lcd = Default::default();
        return lcd;
    }

    fn interrupt_enabled(&self, int: u8, mm: &mem::MemoryMap) -> bool {
        self.stat & int > 0 && mm.interrupt_master_enable
    }

    pub fn run(&mut self, mm: &mut mem::MemoryMap, cycles: u32) {
        //trace!("{:?}", self);
        self.cycles += cycles;
        match self.stat & LCD_STATUS_MODE {
            0 => {
                if self.cycles > 201 {
                    self.cycles -= 201;
                    self.stat &= !3;
                    self.stat |= 2;
                    if self.interrupt_enabled(LCD_STATUS_MODE_2_OAM_INTERRUPT, mm) {
                        mm.interrupt_flag |= interrupt::INTERRUPT_LCD_STAT;
                    }
                }
            },
            2 => {
                if self.cycles > 77 {
                    self.cycles -= 77;
                    self.stat &= !3;
                    self.stat |= 3;
                }
            },
            3 => {
                if self.cycles > 169 {
                    self.cycles -= 169;
                    self.stat &= !3;
                    self.ly = self.ly.wrapping_add(1);
                    if self.interrupt_enabled(LCD_STATUS_LY_COINCIDENCE_INTERRUPT, mm) && self.ly == self.lyc {
                        mm.interrupt_flag |= interrupt::INTERRUPT_LCD_STAT;
                    }
                    if self.ly >= 144 {
                        if self.interrupt_enabled(LCD_STATUS_MODE_1_VBLANK_INTERRUPT, mm) {
                            mm.interrupt_flag |= interrupt::INTERRUPT_LCD_STAT;
                        }
                        if mm.interrupt_master_enable {
                            mm.interrupt_flag |= interrupt::INTERRUPT_VBLANK;
                        }
                        self.stat |= 1;
                    } else {
                        if self.interrupt_enabled(LCD_STATUS_MODE_0_HBLANK_INTERRUPT, mm) {
                            mm.interrupt_flag |= interrupt::INTERRUPT_LCD_STAT;
                        }
                    }
                }
            },
            1 => {
                if self.cycles > 456 {
                    self.cycles -= 456;
                    self.ly = self.ly.wrapping_add(1);
                    if self.interrupt_enabled(LCD_STATUS_LY_COINCIDENCE_INTERRUPT, mm) && self.ly == self.lyc {
                        mm.interrupt_flag |= interrupt::INTERRUPT_LCD_STAT;
                    }
                    if self.ly == 0 {
                        self.stat &= !3;
                        if self.interrupt_enabled(LCD_STATUS_MODE_0_HBLANK_INTERRUPT, mm) {
                            mm.interrupt_flag |= interrupt::INTERRUPT_LCD_STAT;
                        }
                    }
                }
            },
            _ => {
                panic!("bad lcd status {}", self.stat & LCD_STATUS_MODE);
            },
        }
    }
}

#[test]
fn test_lcd() {
    let lcd = Lcd::new();
    assert_eq!(lcd.ctl, 0);
}
