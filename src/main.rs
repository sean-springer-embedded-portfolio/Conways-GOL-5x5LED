#![no_main]
#![no_std]

mod life;

use cortex_m_rt::entry;
use embedded_hal::digital::InputPin;
use microbit::hal::gpio::p0::{P0_14, P0_23};
use microbit::{Board, display::blocking::Display, hal::Rng, hal::timer::Timer};

use panic_rtt_target as _;
use rtt_target::rtt_init_print;

const ROW_COUNT: usize = 5;
type LEDState = [[u8; ROW_COUNT]; ROW_COUNT];
const REFRESH_RATE_MS: u32 = 100; // Spec 1: 10 frames per second refresh rate (100ms)
const DEATH_RESET_RATE_MS: u32 = 500; // per Spec 5: a "dead" state waits 5 frames (500ms)
const COMPLEMENT_RESET_RATE_MS: u32 = 500; // per Spec 4: a complement action can only occur 1 time per 5 frams (500ms)

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

        //extract bit at ith position as 1 or 0 then cast as u8
        let bit: u8 = ((random_number & 1 << i) >> i) as u8;
        state[row][col] = bit;
    }
}

fn complement_state(state: &mut LEDState) {
    for row in state.iter_mut() {
        for item in row.iter_mut() {
            *item ^= 1;
        }
    }
}

struct ResetTimer {
    total: u32,
    current: u32,
}

impl ResetTimer {
    fn new(frames: u32, start: u32) -> Self {
        ResetTimer {
            total: frames,
            current: start,
        }
    }

    fn reset(&mut self) {
        self.current = 0;
    }

    fn tick(&mut self, reset_if_finished: bool) -> bool {
        self.current += 1;

        // prevent possible overflow
        if self.current > self.total {
            self.current = self.total;
        }

        let is_done = self.current == self.total;

        if is_done && reset_if_finished {
            self.reset();
        }

        is_done
    }

    fn finished(&self) -> bool {
        self.current == self.total
    }
}

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let board = Board::take().unwrap();
    let mut timer = Timer::new(board.TIMER0);
    let mut display = Display::new(board.display_pins);
    let mut random_gen = Rng::new(board.RNG);
    let mut reset_timer = ResetTimer::new(DEATH_RESET_RATE_MS / REFRESH_RATE_MS, 0);
    let mut complement_timer = ResetTimer::new(
        COMPLEMENT_RESET_RATE_MS / REFRESH_RATE_MS,
        COMPLEMENT_RESET_RATE_MS / REFRESH_RATE_MS,
    );

    // Configure buttons
    let mut button_a = board.buttons.button_a;
    let mut button_b = board.buttons.button_b;

    let mut state: LEDState = [[0; 5]; 5]; // initialize to all zeros
    randomize_state(&mut random_gen, &mut state); //Spec 2: starts with a random board

    loop {
        display.show(&mut timer, state, REFRESH_RATE_MS);

        if button_a.pressed() {
            reset_timer.reset();
            randomize_state(&mut random_gen, &mut state); //Spec 3: while btn A pressed, randomize every frame
        } else if button_b.pressed() {
            reset_timer.reset();

            //Spec 4: If B btn pressed, complement state, then ignore B btn for 5 frames
            if complement_timer.finished() {
                complement_state(&mut state);
                complement_timer.reset();
            }
        } else if life::done(&state) {
            // Spec 5: if all cells "dead", count 5 frames. If no user input after 5 frames, randomize state
            if reset_timer.tick(true) {
                randomize_state(&mut random_gen, &mut state);
            }
        } else {
            // Spec 6: If not A btn press, not B btn press, and not all cells "dead", take GOL step
            reset_timer.reset();
            life::life(&mut state);
        }

        // tick complement_timer: at least 5 frames between complement action
        complement_timer.tick(false);
    }
}
