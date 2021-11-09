use keyberon::{key_code::KbHidReport, layout::Layout};
use embedded_hal::digital::v2::{InputPin, OutputPin};
use crate::{SiberonPassive, ROWS, COLS, layout::LAYERS};
pub struct SiberonActive<C, R>
where
    C: InputPin,
    R: OutputPin,
{
    passive: SiberonPassive<C, R>,
    layout: Layout<()>,
}

impl<C, R, E: 'static> SiberonActive<C, R>
where
    C: InputPin<Error = E>,
    R: OutputPin<Error = E>,
{
    pub fn init(cols: [C; COLS], rows: [R; ROWS]) -> Result<Self, E>
    {
        let passive = SiberonPassive::init(cols, rows)?;
        let layout = Layout::new(LAYERS);
        Ok(Self{passive, layout})
    }
    pub fn report(&self) -> KbHidReport {
        self.layout.keycodes().collect()
    }
    pub fn poll(&mut self) -> Result<(), E>
    {
        for event in self.passive.events()?
        {
            self.layout.event(event);
        }
        Ok(())
    }
}
