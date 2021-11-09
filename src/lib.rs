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

const HALF_COLS: usize = 6;
const COLS: usize = HALF_COLS*2;
const ROWS: usize = 6;

pub struct SiberonPassive<C, R>
where
    C: InputPin,
    R: OutputPin,
{
    pub matrix: Matrix<C, R, HALF_COLS, ROWS>,
    pub debouncer: Debouncer<PressedKeys::<HALF_COLS, ROWS>>,
}

impl<C, R, E: 'static> SiberonPassive<C, R>
where
    C: InputPin<Error = E>,
    R: OutputPin<Error = E>,
{
    pub fn init(cols: [C; HALF_COLS], rows: [R; ROWS]) -> Result<Self, E>
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
