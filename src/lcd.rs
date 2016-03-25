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

const OAM_OBJ_TO_BG_PRIORITY : u8 = 1<<7;
const OAM_Y_FLIP             : u8 = 1<<6;
const OAM_X_FLIP             : u8 = 1<<5;
const OAM_PALETTE_NUMBER     : u8 = 1<<4;

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
        self.stat & int > 0
    }

    fn put_pixel(&self,
                 mm: &mut mem::MemoryMap,
                 pixels: &mut [u8; 160*144], x: i32, y: i32,
                 color: u8, oam: bool) {
        if x < 0 || y < 0 || y >= 144 || x >= 160 {
            return;
        }
        if y != self.ly as i32 {
            return;
        }
        pixels[y as usize * 160 + x as usize] = match color {
            0 => { 0b111_111_11 }
            1 => { 0b100_100_10 }
            2 => { 0b010_010_01 }
            3 => { 0b000_000_00 }
            _ => { panic!("bad color {}", color); }
        };
    }

    fn draw_tile(&self,
                 mm: &mut mem::MemoryMap,
                 pixels: &mut [u8; 160*144], x: i32, y: i32,
                 tile_start_addr: u16,
                 palette: [u8; 4], oam_flags: u8, oam: bool) {
        for j in 0..8 {
            let l = mm.read(j*2 + tile_start_addr);
            let h = mm.read(j*2 + tile_start_addr + 1);
            for k in 0..8 {
                let p = (((h & (1<<k)) >> k) << 1) | ((l & (1<<k)) >> k);
                let xpos = if (oam_flags & OAM_X_FLIP) > 0 { x + k as i32 } else { x + 7 - k as i32 };
                let ypos = if (oam_flags & OAM_Y_FLIP) > 0 { y + 7 - j as i32 } else { y + j as i32 };
                if p == 0 && oam {
                    continue
                }
                self.put_pixel(mm, pixels, xpos, ypos, palette[p as usize], oam);
            }
        }
    }

    pub fn draw_tiles(&self, mm: &mut mem::MemoryMap, pixels: &mut [u8; 160*144]) {
        let palette : [u8; 4] = [
            (self.obp0 & 0x03),
            (self.obp0 & 0x0c) >> 2,
            (self.obp0 & 0x30) >> 4,
            (self.obp0 & 0xc0) >> 6,
            ];

        let mut tile_start_addr = 0x8000;
        for j in 0..12 {
            for i in 0..16 {
                self.draw_tile(mm, pixels, i as i32 * 8, j as i32 * 8, tile_start_addr, palette, 0, false);
                tile_start_addr += 16;
            }
        }
    }

    fn get_tile_map_addr(&self) -> u16 {
        if (self.ctl & LCD_CTL_BG_TILE_MAP_DISPLAY_SELECT) > 0 {
            0x9c00
        } else {
            0x9800
        }
    }

    fn get_window_tile_map_addr(&self) -> u16 {
        if (self.ctl & LCD_CTL_WINDOW_TILE_MAP_DISPLAY_SELECT) > 0 {
            0x9c00
        } else {
            0x9800
        }
    }

    fn get_tile_start_addr(&self, tile: u8) -> u16 {
        if (self.ctl & LCD_CTL_BG_WINDOW_TILE_DATA_SELECT) > 0 {
            0x8000 + tile as u16 * 16
        } else {
            0x9000u16.wrapping_add(((tile as i8) as i16 * 16) as u16)
        }
    }

    fn draw_bg(&self, mm: &mut mem::MemoryMap, pixels: &mut [u8; 160*144]) {
        let palette : [u8; 4] = [
            (self.bgp & 0x03),
            (self.bgp & 0x0c) >> 2,
            (self.bgp & 0x30) >> 4,
            (self.bgp & 0xc0) >> 6,
            ];

        //println!("ly={} palette={:?}", self.ly, palette);

        //println!("ctl={:02x} scx={} scy={}", self.ctl, self.scx, self.scy);
        if self.ctl & LCD_CTL_BG_DISPLAY == 0 {
            return;
        }

        let tile_map_addr = self.get_tile_map_addr();
        //println!("ly={} palette={:?} addr={:04x} tile_start={:04x}", self.ly, palette, tile_map_addr, self.get_tile_start_addr(0));

        for j in 0..19 {
            for i in 0..21 {
                let tile_pos_x = ((i + self.scx / 8) % 32) as u16;
                let tile_pos_y = ((j + self.scy / 8) % 32) as u16;
                let myaddr = tile_map_addr + tile_pos_y * 32 + tile_pos_x;
                let tile = mm.read(myaddr);
                let tile_start_addr = self.get_tile_start_addr(tile);
                let x = i as i32 * 8 - (self.scx % 8) as i32;
                let y = j as i32 * 8 - (self.scy % 8) as i32;
                if self.ly as i32 >= y && self.ly as i32 <= y + 8 {
                        //println!("tile_pos_x={:02x} tile_pos_y={:02x} tile={:02x} tile_start_addr={:04x} x={} y={} myaddr={:04x}",
                        //         tile_pos_x, tile_pos_y, tile, tile_start_addr, x, y, myaddr);
                    self.draw_tile(mm, pixels, x as i32, y as i32, tile_start_addr, palette, 0, false);
                }
            }
        }
    }

    fn draw_window(&self, mm: &mut mem::MemoryMap, pixels: &mut [u8; 160*144]) {
        if (self.ctl & LCD_CTL_WINDOW_DISPLAY_ENABLE) == 0 {
            return;
        }

        let palette : [u8; 4] = [
            (self.bgp & 0x03),
            (self.bgp & 0x0c) >> 2,
            (self.bgp & 0x30) >> 4,
            (self.bgp & 0xc0) >> 6,
            ];

        let tile_map_addr = self.get_window_tile_map_addr();

        for j in 0..19 {
            for i in 0..21 {
                let tile_pos_x = (i % 32) as u16;
                let tile_pos_y = (j % 32) as u16;
                let myaddr = tile_map_addr + tile_pos_y * 32 + tile_pos_x;
                let tile = mm.read(myaddr);
                let tile_start_addr = self.get_tile_start_addr(tile);
                let x = i as i32 * 8 + self.wx as i32 - 6;
                let y = j as i32 * 8 + self.wy as i32;
                if self.ly as i32 >= y && self.ly as i32 <= y + 8 {
                    self.draw_tile(mm, pixels, x as i32, y as i32, tile_start_addr, palette, 0, false);
                }
            }
        }
    }

    fn draw_oam_tile(&self, mm: &mut mem::MemoryMap, pixels: &mut [u8; 160*144], x: u8, y: u8, tile: u8, flags: u8) {
        let tile_start_addr = 0x8000 + tile as u16 * 16;
        let obp = if flags & OAM_PALETTE_NUMBER > 0 {
            self.obp1
        } else {
            self.obp0
        };
        let palette : [u8; 4] = [
            (obp & 0x03),
            (obp & 0x0c) >> 2,
            (obp & 0x30) >> 4,
            (obp & 0xc0) >> 6,
        ];
        if self.ly as i32 >= y as i32 - 16 && self.ly as i32 <= y as i32 - 8 {
            self.draw_tile(mm, pixels, x as i32 - 8, y as i32 - 16, tile_start_addr, palette, flags, true);
        }
    }

    fn draw_oam(&self, mm: &mut mem::MemoryMap, pixels: &mut [u8; 160*144]) {
        let is_8x8 = (self.ctl & LCD_CTL_OBJ_SIZE) == 0;

        for i in 0..40 {
            let y     = mm.read(0xfe00 + i*4 + 0);
            let x     = mm.read(0xfe00 + i*4 + 1);
            let tile  = mm.read(0xfe00 + i*4 + 2);
            let flags = mm.read(0xfe00 + i*4 + 3);

            if y >= 160 {
                continue;
            }

            if is_8x8 {
                self.draw_oam_tile(mm, pixels, x, y, tile, flags);
            } else {
                if (flags & OAM_Y_FLIP) > 0 {
                    self.draw_oam_tile(mm, pixels, x, y + 8, tile & 0xfe, flags);
                    self.draw_oam_tile(mm, pixels, x, y,     tile | 0x01, flags);
                } else {
                    self.draw_oam_tile(mm, pixels, x, y,     tile & 0xfe, flags);
                    self.draw_oam_tile(mm, pixels, x, y + 8, tile | 0x01, flags);
                }
            }
        }
    }

    pub fn draw(&self, mm: &mut mem::MemoryMap, pixels: &mut [u8; 160*144]) {
        if (self.ctl & LCD_CTL_ENABLE) == 0 {
            return;
        }

        self.draw_bg(mm, pixels);
        self.draw_window(mm, pixels);
        self.draw_oam(mm, pixels);
    }

    pub fn run(&mut self, mm: &mut mem::MemoryMap, cycles: u32, pixels: &mut [u8; 160*144]) -> bool {
        //println!("{:?}", self);
        let prev_ly = self.ly;
        let mut vblank = false;
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
                        vblank = true;
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

        if prev_ly != self.ly {
            // draw new scanline
            self.draw(mm, pixels);
        }
        return vblank;
    }
}

#[test]
fn test_lcd() {
    let lcd = Lcd::new();
    assert_eq!(lcd.ctl, 0);
}
