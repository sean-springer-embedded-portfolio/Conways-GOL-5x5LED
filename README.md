# Conways-GOL-5x5LED

Copyright (c) 2026 Sean Springer

**Conways-GOL-5x5LED** Plays Conway's Game of Life (GOL) using the 5x5 LED grid on the Microbit V2 (MB2)  
and is written entirely in `embeded Rust` using `#![no_std]`. 

## User Interface (UI)

The program will begin (after `flashing` to the MB2, see _Build and Run_ below) with a random starting state and progress  
according to the standard GOL rules (see _GOL Rules_). The game will progress at **100ms** refresh rate according to the GOL rules until either of the  
MB2 buttons are pressed (see below) or until all LEDs have become unlit (a "dead" GOL state). In the event of the later,  
a **500ms** timer will begin and, in-lieu of any button press, upon expiration the GOL will be assigned another random state  
and start again. 

The main user interactions implemented for this program use the two buttons marked `A` (bus 0, pin 14) and `B` (bus 0, pin 23)  
located on the same side of the MB2 as the LED grid. The `A` button will re-randomize the board while pressed and the `B` button will flip  
the current state of the LEDs (on->off and off->on). Note, however, that there is a **500ms** cooldown period implemented on the `B` button's  
action.

The following list summarizes the UI and gameplay evolution as described above:
1. The program runs at 10 frames per second (100ms) refresh rate
2. The program begins with a random board
3. While the A btn is pressed, the board will re-randomize with every frame
4. If the B btn is pressed, the board is complemented (on->off and off->on) but there will be a 500ms cooldown period after each complement
5. If all LEDs are off, the program will wait up to 5 frames (500ms) and, if no other btn is pressed, will re-randomize and continue
6. Otherwise, the standard GOL steps are taken with each frame (100ms)

## Mechanics

Randomizations are assigned using the MB2 hardware random number generator ([RNG](https://docs.rs/microbit/latest/microbit/hal/rng/index.html)).  
The RNG is used to populate a `u32` whereby each LED is toggled on or off according to the right most 25 bits of this `u32` number.  
The bit mapping progresses starting at the top left LED (the first bit) to the bottom right LED (the 25th bit). 

The state-flip (complement) action is implemented using a bit-wise-like `XOR` operation on the current state of each LED. 

The display state of the LEDs for each frame is perfomed using the BSP `display::blocking` module whereby the blocking display is lit  
for the **100ms** refresh rate.  

## GOL Rules

The typical [GOL rules](https://playgameoflife.com/) for evolution have been implemented on the 5x5 MB2 LED grid.  
In short, the evolutionary rules enforced here (where "active" indicates a lit LED and "dead" indicates a non-lit LED) are:
1. An active cell with only 1 or fewer neighbors dies (as if by solitude)
2. An active cell with 4 or more neighbors dies (as if by overpopulation)
3. An active cell with 2 or 3 neighbors remains unchanged
4. A dead cell with 3 neighbors is brought back to life

## Build and Run

Assuming you have an attached MB2 with necessary permissions (see [Rust MB2 Discovery Book](https://docs.rust-embedded.org/discovery-mb2/))  
then this program can be `flashed` onto the MB2 nRF52820 using

```bash
cargo embed --release
```

## Sources
1. [Rust MB2 Discovery Book](https://docs.rust-embedded.org/discovery-mb2/)
2. [Conway's Game of Life](https://playgameoflife.com/)
3. [Rustdoc](https://doc.rust-lang.org/rustdoc/what-is-rustdoc.html)
4. Claude Sonnet 4.5 (free version)
5. nRF52833 Product Specification v1.6
6. MicroBit v2.21 Schematic
7. [Microbit Hal Docs](https://docs.rs/microbit/latest/microbit/hal/index.html)

## License

This program is licensed under the "MIT License". Please  
see the file `LICENSE` in the source distribution of this  
software for license terms.