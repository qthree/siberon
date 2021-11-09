#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::prelude::*;
//use embedded_hal::serial::Read;

use siberon::{SiberonPassive, Event};

macro_rules! input_pins {
    ($($gpio:expr => [$($pin:ident),*]),*) => {
        [$(
            $(
                $gpio.$pin.into_pull_up_input().downgrade(),
            )*
        )*]
    }
}
macro_rules! output_pins {
    ($($gpio:expr => [$($pin:ident),*]),*) => {
        [$(
            $(
                $gpio.$pin.into_output_high().downgrade(),
            )*
        )*]
    }
}

struct Foo<const BAR: usize>([bool; BAR]);

impl<const BAR: usize> core::cmp::PartialEq for Foo<BAR> {
    fn eq(&self, other: &Self) -> bool {
        self.0.iter()
            .zip(other.0.iter())
            .all(|(this, other)| this == other)
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    /*
     * For examples (and inspiration), head to
     *
     *     https://github.com/Rahix/avr-hal/tree/next/examples
     *
     * NOTE: Not all examples were ported to all boards!  There is a good chance though, that code
     * for a different board can be adapted for yours.  The Arduino Uno currently has the most
     * examples available.
     */


    let cols = input_pins!(pins => [d13, d12, d11, d10, d9, d8]);
    let rows = output_pins!(pins => [d7, d6, d5, d4, d3, d2]);

    //let mut led = pins.d13.into_output();
    
    let mut siberon = SiberonPassive::init(cols, rows).unwrap();

    let mut old_keys = siberon::keyberon::matrix::PressedKeys::<6, 6>::default();

    loop {
        //led.toggle();
        //arduino_hal::delay_ms(1000);

        /*match siberon.events() {
            Ok(events) => {
                for event in events {
                    let (event, col, row) = match event {
                        Event::Press(col, row) => {
                            ('P', col, row)
                        }
                        Event::Release(col, row) => {
                            ('R', col, row)
                        }
                    };
                    ufmt::uwriteln!(&mut serial, "\r{} {} {}", event, col, row).void_unwrap();
                }
            }
            Err(_err) => {
                ufmt::uwriteln!(&mut serial, "Err").void_unwrap();
            }
        }*/
        match siberon.events_serilized() {
            Ok(events) => {
                for event in events {
                    for char in &event {
                        serial.write_char(*char as char).void_unwrap();
                    }
                }
            },
            Err(err) => {
                ufmt::uwriteln!(&mut serial, "Err").void_unwrap();
            }
        }
        arduino_hal::delay_ms(1);
    }
}
