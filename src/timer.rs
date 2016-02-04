use std::fmt;
use cpu;
use mem;
use interrupt;

#[derive(Default)]
pub struct Timer {
	pub div : u8,  // Divider Register (R/W)
	pub tima : u8, // Timer Counter (R/W)
	pub tma : u8,  // Timer Modulo (R/W)
	pub tac : u8,  // Timer Control (R/W)
	last_tick : u32,
	last_div_tick : u32,
}

const TIMER_TAC_TIMER_STOP         : u8 = 1<<2;        // (0=Stop, 1=Start)
const TIMER_TAC_INPUT_CLOCK_SELECT : u8 = 1<<1 | 1<<0; // 00:   4096 Hz
                                                       // 01: 262144 Hz
                                                       // 10:  65536 Hz
                                                       // 11:  16384 Hz

impl fmt::Debug for Timer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Timer {{ div:{:02x} tima:{:02x} tma:{:02x} tac:{:02x} \
               last_tick:{:02x} last_div_tick:{:02x} }}",
               self.div, self.tima, self.tma, self.tac, self.last_tick,
               self.last_div_tick)
    }
}


impl Timer {
    pub fn new() -> Timer {
        let timer: Timer = Default::default();
        return timer;
    }

    pub fn run(&mut self, mm: &mut mem::MemoryMap, cycles: u32) {
        if self.tac & TIMER_TAC_TIMER_STOP == 0 {
            return
        }

        let cycles_per_tick = match self.tac & TIMER_TAC_INPUT_CLOCK_SELECT {
            0 => 1024,
            1 => 16,
            2 => 64,
            3 => 25,
            _ => panic!("bad tac {}", self.tac),
        };

        // increment tima based on selected clock
        self.last_tick += cycles;
        if self.last_tick >= cycles_per_tick {
            self.last_tick -= cycles_per_tick;
            self.tima = self.tima.wrapping_add(1);

            // handle overflow
            if self.tima == 0 {
                self.tima = self.tma;
                if mm.interrupt_master_enable {
                    mm.interrupt_flag |= interrupt::INTERRUPT_TIMER;
                }
            }
        }

        // increment div (always 16384 Hz)
        self.last_div_tick += cycles;
        if self.last_div_tick >= 256 {
            self.last_div_tick -= 256;
            self.div = self.div.wrapping_add(1);
        }
    }
}

#[test]
fn test_timer() {
    let timer = Timer::new();
    assert_eq!(timer.div, 0);
}
