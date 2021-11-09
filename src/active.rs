use keyberon::{key_code::KbHidReport, layout::{Layout}};
use embedded_hal::digital::v2::{InputPin, OutputPin};
use crate::{SiberonPassive, ROWS, HALF_COLS, layout::LAYERS, Event};
pub struct SiberonActive<C, R>
where
    C: InputPin,
    R: OutputPin,
{
    passive: SiberonPassive<C, R>,
    layout: Layout<()>,
}

pub type CustomEvent = keyberon::layout::CustomEvent<()>;

impl<C, R, E: 'static> SiberonActive<C, R>
where
    C: InputPin<Error = E>,
    R: OutputPin<Error = E>,
{
    pub fn init(cols: [C; HALF_COLS], rows: [R; ROWS]) -> Result<Self, E>
    {
        let passive = SiberonPassive::init(cols, rows)?;
        let layout = Layout::new(LAYERS);
        Ok(Self{passive, layout})
    }
    pub fn report(&self) -> KbHidReport {
        self.layout.keycodes().collect()
    }
    pub fn process_events(&mut self) -> Result<(), E>
    {
        for event in self.passive.events()?
        {
            self.layout.event(event);
        }
        Ok(())
    }
    pub fn handle_event(&mut self, event: Event) {
        self.layout.event(event);
    }
    pub fn tick(&mut self) -> CustomEvent {
        self.layout.tick()
    }
    pub fn poll(&mut self) -> Result<Poll, E> {
        self.process_events()?;
        let custom_event = self.tick();
        let report = self.report();
        Ok(Poll{
            custom_event, report
        })
    }
}

#[derive(Debug)]
pub struct Poll {
    pub custom_event: CustomEvent,
    pub report: KbHidReport,
}

fn de(bytes: &[u8], (dj, di): (u8, u8)) -> Result<Event, ()> {
    match *bytes {
        [b'P', i, j, b'\n'] => Ok(Event::Press(de_digit(i, di)?, de_digit(j, dj)?)),
        [b'R', i, j, b'\n'] => Ok(Event::Release(de_digit(i, di)?, de_digit(j, dj)?)),
        _ => Err(()),
    }
}

fn de_digit(byte: u8, shift: u8) -> Result<u8, ()> {
    if byte >= b'0' && byte <= b'9' {
        Ok((byte - b'0') + shift)
    } else {
        Err(())
    }
}

#[derive(Default)]
pub struct DeBuffer {
    buf: [u8; 4],
    shift: (u8, u8),
}
impl DeBuffer {
    pub const fn new(shift: (u8, u8)) -> Self {
        Self{buf: [0; 4], shift}
    }
    pub fn feed(&mut self, byte: u8) -> Option<Event>{
        self.buf.rotate_left(1);
        self.buf[3] = byte;

        if self.buf[3] == b'\n' {
            if let Ok(event) = de(&self.buf[..], self.shift) {
                return Some(event);
            }
        }
        None
    }
}