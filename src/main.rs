//! Main.rs
//! Copyright © 2026 Sean Springer
//! [This program is licensed under the "MIT License"]
//! Please see the file LICENSE in the source distribution of this software for license terms.
//!
//! Play Conway's Game of Life (GOL) on the Microbit V2 (MB2) 5x5 LED matrix
//!
//! The Rust code present here can be summarized as follows (top to bottom order):
//! 1. Defines a set of constants dictating the LED board size and refresh rates
//! 2. Defines and implements a convience trait for the 2 button Microbit InputPin structs
//! 3. Defines helper functions which randomize or complement the current board state
//! 4. Defines and implements a helper struct for simplifying the the refresh rate criteria
//!    (see below for more info on this)
//! 5. Defines the Microbit entry point event loop where
//!     - The required MB2 peripherals are captured
//!     - States are initialized
//!     - Event loop with UI (btn controls) begins
//!
//! This implementation of the Game of Life and UI obeys the following Specifications:
//! 1. The display refresh rate is 100ms (10 frames per second)
//! 2. The GOL is initialized to a random state
//! 3. While the MB2 A btn is pressed, the state will be re-randomized
//! 4. If the B btn is pressed, the state will be complimented (on -> off and off -> on).
//!    A 500ms cooldown period will occur between every compliment action
//! 5. If the GOL state is all zeros ("dead" state), then a 500ms timer will begin.
//!    If no other btn is pressed during that 500ms, the GOL restarts with a random starting state
//! 6. Otherwise a normal GOL step is taken according to Conway's GOL rules

#![no_main]
#![no_std]

mod life;

use cortex_m_rt::entry;
use embedded_hal::digital::InputPin;
use microbit::hal::gpio::p0::{P0_14, P0_23};
use microbit::{Board, display::blocking::Display, hal::Rng, hal::timer::Timer};

use panic_rtt_target as _;
use rtt_target::rtt_init_print;

/// The MB2 has 5 LED rows and 5 LED columns
const ROW_COUNT: usize = 5;
/// Type definition defining the LEDState to be a 5x5 array of u8 variables
type LEDState = [[u8; ROW_COUNT]; ROW_COUNT];
/// Spec 1: 10 frames per second refresh rate (100ms)
const REFRESH_RATE_MS: u32 = 100;
/// Per Spec 5: a "dead" state waits 5 frames (500ms)
const DEATH_RESET_RATE_MS: u32 = 500;
/// Per Spec 4: a complement action can only occur 1 time per 5 frams (500ms)
const COMPLEMENT_RESET_RATE_MS: u32 = 500;

/// ButtonPress Trait
///
/// Defines a convience trait that can extend the methods available to the HAL GPIO pins.
/// Requires that the implementors of the ButtonPress trait also implement the Hal::digital::InputPin
/// trait for meaningful implementation
trait ButtonPress: InputPin {
    /// fn pressed(&mut self) -> bool : Abstract!
    ///
    /// Must be defined by the implementor. Should return true if the button is pressed and false otherwise.
    /// Alternatively, this function could be interpreted as returning true if the user is influence the
    /// InputPin to be in a state other than it's Reset state
    fn pressed(&mut self) -> bool;
}

/// Implementation of ButtonPress trait for bus 0, pin 14 (the A btn)
impl<T> ButtonPress for P0_14<T>
where
    P0_14<T>: InputPin,
{
    /// Returns true if the voltage on the bus 0 pin 14 is equal to ground.
    /// The A btn is pressed when the voltage is equal to ground because this btn is a Momentary switch
    /// (Normally Open) and so the pressed state completes the circuit (see the nRF52820 schematic)
    ///
    /// To protect against potential bounce problems, the voltage state is queried 3 times
    fn pressed(&mut self) -> bool {
        // protect against bounce:
        self.is_low().unwrap() & self.is_low().unwrap() & self.is_low().unwrap()
    }
}

/// Implementation of ButtonPress trait for bus 0, pin 23 (the B btn)
impl<T> ButtonPress for P0_23<T>
where
    P0_23<T>: InputPin,
{
    /// Returns true if the voltage on the bus 0 pin 23 is equal to ground.
    /// The B btn is pressed when the voltage is equal to ground because this btn is a Momentary switch
    /// (Normally Open) and so the pressed state completes the circuit (see the nRF52820 schematic)
    ///
    /// To protect against potential bounce problems, the voltage state is queried 3 times
    fn pressed(&mut self) -> bool {
        // protect against bounce:
        self.is_low().unwrap() & self.is_low().unwrap() & self.is_low().unwrap()
    }
}

/// fn randomize_state(&mut Rng, &mut LEDState)
///
/// Takes a mutable reference to the Hal hardware random number generator (Rng) and
/// a mutable references to the 5x5 array LEDState which is altered in-place.
///
/// A random u32 is drawn from the MB2 random number generator and is used to set
/// the LEDState array by taking the right-most 25 bits (25 MSB on an LSB architecture)
/// and assigning them to the LEDState in order (top-left to bottom-right).
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

/// fn complement_state(&mut LEDState)
///
/// Takes a mutable reference to the current LEDState and alters it in-place
///
/// Given the current LEDState, iterate through each LED Diode and flip its state
/// (on->off and off->on). Each LED is mutably iterated through and its state is
/// complemented using XOR boolean logic
fn complement_state(state: &mut LEDState) {
    for row in state.iter_mut() {
        for item in row.iter_mut() {
            *item ^= 1;
        }
    }
}

/// ResetTimer Struct
///
/// The ResetTimer struct tracks a current loop count (multiple of the REFRESH_RATE_MS) and a
/// total loop count (also a multiple of REFRESH_RATE_MS) to determine when a period of time has elapsed.
struct ResetTimer {
    total: u32,
    current: u32,
}

/// Implt ResetTimer
///
/// Provides method to initalize the reset timer, reset its counting, update the clock,
/// and check if the timer has expired
impl ResetTimer {
    /// fn new(u32, u32) -> Self
    ///
    /// Returns a new ResetTimer instance initialized to frames total seconds (the expiration time)
    /// and initialized to a current start time. The start time will likely be set to 0 but can be set
    /// to some other number (eg equal to frames) which can provide different inital poll behavior
    fn new(frames: u32, start: u32) -> Self {
        ResetTimer {
            total: frames,
            current: start,
        }
    }

    /// fn reset(&mut self)
    ///
    /// reset the timer to it's starting state (furthest from expired)
    fn reset(&mut self) {
        self.current = 0;
    }

    /// fn tick(&mut self, bool) -> bool
    ///
    /// This method will update the timer's count, returning true if this update
    /// has caused the timer to expire and false otherwise. If reset_if_finished is true,
    /// then the internal timer state will reset if this function returns true
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

    /// fn finsihed(&self) -> bool
    ///
    /// This method will return true if the timer has expired and false otherwise
    fn finished(&self) -> bool {
        self.current == self.total
    }
}

/// Main entry point for the MB2
///
/// The following outlines the steps process of this embeded program:
/// 1. Initialize structs and grab handles to MB2 peripherals that will be used
/// 2. Initialize the LED GOL state to a random starting board
/// 3. Event Loop
///     1. Display the GOL state on the LEDs for REFRESH_RATE_MS duration
///     2. If A btn is pressed, re-randomize the GOL state
///     3. Else if B btn is pressed and the complement_timer has expired, complement the current board
///        (and reset the complement timer). If complement_timer has not expired then the GOL state remains unchanged
///     4. If the GOL state is done ("dead") and the reset_timer is expired, re-randomize the GOL state. If the
///        reset_timer is not expired then the GOL state remains unchanged
///     5. If 2-4 have not occured during this frame, then a GOL step is taken to update the GOL state as defined
///        in life.rs module
///     6. The compelent_timer is updated every frame (note the rest_timer is only updated each "dead" frame)
#[entry]
fn main() -> ! {
    rtt_init_print!();

    // initialize structs and grab handles to MB2 peripherals
    let board = Board::take().unwrap();
    let mut timer = Timer::new(board.TIMER0);
    let mut display = Display::new(board.display_pins);
    let mut random_gen = Rng::new(board.RNG); //hardware trigger
    let mut reset_timer = ResetTimer::new(DEATH_RESET_RATE_MS / REFRESH_RATE_MS, 0);
    let mut complement_timer = ResetTimer::new(
        COMPLEMENT_RESET_RATE_MS / REFRESH_RATE_MS,
        COMPLEMENT_RESET_RATE_MS / REFRESH_RATE_MS,
    ); // initialized to a finished() == true state

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
