#![no_main]
#![no_std]

mod life;
use life::{done, life};

use cortex_m_rt::entry;
//use embedded_hal::digital::InputPin;
use microbit::hal::timer::Timer;
use microbit::pac::pwm0::seq::REFRESH;
use microbit::{Board, display::blocking::Display};
use panic_rtt_target as _;
use rtt_target::rtt_init_print;

type LEDState = [[u8; 5]; 5];
const REFRESH_RATE_MS: u32 = 100;

fn randomize_state(state: &mut LEDState) {
    //todo!
    state[1][1] = 1u8;
}

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let board = Board::take().unwrap();
    let mut timer = Timer::new(board.TIMER0);
    let mut display = Display::new(board.display_pins);
    // Configure buttons
    let mut button_a = board.buttons.button_a;
    let mut button_b = board.buttons.button_b;

    let mut state: LEDState = [[0; 5]; 5];
    randomize_state(&mut state);

    loop {
        display.show(&mut timer, state, REFRESH_RATE_MS);
    }
}
