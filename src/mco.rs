use crate::gpio::{gpioa, AltMode, Floating, Input};

#[derive(Clone, Copy)]
pub enum MCODiv {
    Div1 = 0,
    Div2 = 1,
    Div4 = 2,
    Div8 = 3,
    Div16 = 4,
}

#[derive(Clone, Copy)]
pub enum MCOSel {
    None = 0,
    Sysclk = 1,
    Hsi = 2,
    Msi = 3,
    Hse = 4,
    Pll = 5,
    Lsi = 6,
    Lse = 7,
}

pub trait Pin {
    fn into_mco(self);
}

impl Pin for gpioa::PA8<Input<Floating>> {
    fn into_mco(self) {
        self.set_alt_mode(AltMode::SYSTEM);
    }
}
