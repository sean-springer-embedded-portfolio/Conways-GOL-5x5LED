#![no_main]
#![no_std]

mod life;
use life::*;

use cortex_m_rt::entry;
use embedded_hal::digital::InputPin;
//use microbit::hal::timer::Timer;
//use microbit::pac::pwm0::seq::REFRESH;
use microbit::hal::gpio::p0::{P0_14, P0_23};
use microbit::{Board, display::blocking::Display, hal::Rng, hal::timer::Timer};

use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

const ROW_COUNT: usize = 5;
type LEDState = [[u8; ROW_COUNT]; ROW_COUNT];
const REFRESH_RATE_MS: u32 = 100;
const DEATH_RESET_RATE_MS: u32 = 500;

trait ButtonPress: InputPin {
    fn pressed(&mut self) -> bool;
}

impl<T> ButtonPress for P0_14<T>
where
    P0_14<T>: InputPin,
{
    fn pressed(&mut self) -> bool {
        // protect against bounce:
        self.is_low().unwrap() & self.is_low().unwrap() & self.is_low().unwrap()
    }
}

impl<T> ButtonPress for P0_23<T>
where
    P0_23<T>: InputPin,
{
    fn pressed(&mut self) -> bool {
        // protect against bounce:
        self.is_low().unwrap() & self.is_low().unwrap() & self.is_low().unwrap()
    }
}

fn randomize_state(random_gen: &mut Rng, state: &mut LEDState) {
    const LED_COUNT: usize = ROW_COUNT * ROW_COUNT;
    let random_number: u32 = random_gen.random_u32();

    for i in 0..LED_COUNT {
        let row: usize = i / ROW_COUNT;
        let col: usize = i % ROW_COUNT;
        let bit: u8 = ((random_number & 1 << i) >> i) as u8; //extract bit at ith position as 1 or 0, u8

        state[row][col] = bit;
    }
}

fn complement_state(state: &mut LEDState) {
    for i in 0..ROW_COUNT {
        for j in 0..ROW_COUNT {
            state[i][j] ^= 1;
        }
    }
}

//enum ResetTimerType {
//    CountUp(u32),
//    CountDown(u32),
//}
struct ResetTimer {
    total: u32,
    current: u32,
}

impl ResetTimer {
    fn new(frames: u32) -> Self {
        ResetTimer {
            total: frames,
            current: 0,
        }
    }

    fn reset(&mut self) {
        self.current = 0;
    }

    fn tick(&mut self) -> bool {
        self.current += 1;
        let is_done = self.current >= self.total;

        if is_done {
            self.reset();
        }

        is_done
    }
}

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let board = Board::take().unwrap();
    let mut timer = Timer::new(board.TIMER0);
    let mut display = Display::new(board.display_pins);
    let mut random_gen = Rng::new(board.RNG);
    let mut reset_timer = ResetTimer::new(DEATH_RESET_RATE_MS / REFRESH_RATE_MS);
    //let mut complement_timer = ResetTimer::new(500 / REFRESH_RATE_MS);

    // Configure buttons
    let mut button_a = board.buttons.button_a;
    let mut button_b = board.buttons.button_b;

    let mut state: LEDState = [[0; 5]; 5];
    randomize_state(&mut random_gen, &mut state);

    loop {
        display.show(&mut timer, state, REFRESH_RATE_MS);

        if button_a.pressed() {
            reset_timer.reset();
            randomize_state(&mut random_gen, &mut state);
        } else if button_b.pressed() {
            reset_timer.reset();
            complement_state(&mut state);
        } else if life::done(&state) {
            if reset_timer.tick() {
                randomize_state(&mut random_gen, &mut state);
            }
        } else {
            life::life(&mut state);
        }
    }
}
