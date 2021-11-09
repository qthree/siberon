#![deny(unsafe_code)]
#![no_std]

#[cfg(feature = "layout")]
pub mod layout;

#[cfg(feature = "active")]
pub mod active;

pub use keyberon;
pub use keyberon::debounce::Event;

use embedded_hal::digital::v2::{InputPin, OutputPin};
use keyberon::{debounce::Debouncer, matrix::Matrix, matrix::PressedKeys};

const COLS: usize = 6;
const ROWS: usize = 6;

pub struct SiberonPassive<C, R>
where
    C: InputPin,
    R: OutputPin,
{
    pub matrix: Matrix<C, R, COLS, ROWS>,
    pub debouncer: Debouncer<PressedKeys::<COLS, ROWS>>,
}

impl<C, R, E: 'static> SiberonPassive<C, R>
where
    C: InputPin<Error = E>,
    R: OutputPin<Error = E>,
{
    pub fn init(cols: [C; COLS], rows: [R; ROWS]) -> Result<Self, E>
    {
        let matrix = Matrix::new(cols, rows)?;
        let debouncer = Debouncer::new(PressedKeys::default(), PressedKeys::default(), 5);
        Ok(Self{matrix, debouncer})
    }

    pub fn events<'a>(&'a mut self) -> Result<impl 'a + Iterator<Item=Event>, E>
    {
        Ok(self.debouncer.events(self.matrix.get()?))
    }

    pub fn events_serilized<'a>(&'a mut self) -> Result<impl 'a + Iterator<Item=[u8; 4]>, E>
    {
        Ok(self.events()?.map(ser))
    }
}

fn ser(e: Event) -> [u8; 4] {
    match e {
        Event::Press(i, j) => [b'P', i+b'0', j+b'0', b'\n'],
        Event::Release(i, j) => [b'R', i+b'0', j+b'0', b'\n'],
    }
}

fn de(bytes: &[u8]) -> Result<Event, ()> {
    match *bytes {
        [b'P', i, j, b'\n'] => Ok(Event::Press(de_digit(i)?, de_digit(j)?)),
        [b'R', i, j, b'\n'] => Ok(Event::Release(de_digit(i)?, de_digit(j)?)),
        _ => Err(()),
    }
}

fn de_digit(byte: u8) -> Result<u8, ()> {
    if byte >= b'0' && byte <= b'9' {
        Ok(byte - b'0')
    } else {
        Err(())
    }
}

trait ResultExt<T> {
    fn get(self) -> T;
}
impl<T> ResultExt<T> for Result<T, core::convert::Infallible> {
    fn get(self) -> T {
        match self {
            Ok(v) => v,
            Err(e) => match e {},
        }
    }
}
