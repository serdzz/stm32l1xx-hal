use crate::stm32::EXTI;
//use crate::pwr::PowerMode;
use crate::stm32::SYSCFG;

/// Edges that can trigger a configurable interrupt line.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum TriggerEdge {
    /// Trigger on rising edges only.
    Rising,
    /// Trigger on falling edges only.
    Falling,
    /// Trigger on both rising and falling edges.
    Both,
}

/// Higher-lever wrapper around the `EXTI` peripheral.
pub struct ExtiExt {
    raw: EXTI,
}

impl ExtiExt {
    /// Creates a new `Exti` wrapper from the raw `EXTI` peripheral.
    pub fn new(raw: EXTI) -> Self {
        Self { raw }
    }

    /// Destroys this `Exti` instance, returning the raw `EXTI` peripheral.
    pub fn release(self) -> EXTI {
        self.raw
    }

    /// Starts listening on a GPIO interrupt line.
    ///
    /// GPIO interrupt lines are "configurable" lines, meaning that the edges
    /// that should trigger the interrupt can be configured. However, they
    /// require more setup than ordinary "configurable" lines, which requires
    /// access to the `SYSCFG` peripheral.
    // `port` and `line` are almost always constants, so make sure they can get
    // constant-propagated by inlining the method. Saves ~600 Bytes in the
    // `lptim.rs` example.
    #[inline]
    pub fn listen_gpio(&mut self, syscfg: &mut SYSCFG, port: u8, line: u8, edge: TriggerEdge) {
        // translate port into bit values for EXTIn registers
        let port_bm = port;

        unsafe {
            match line {
                0 | 1 | 2 | 3 => {
                    syscfg.exticr1.modify(|_, w| match line {
                        0 => w.exti0().bits(port_bm),
                        1 => w.exti1().bits(port_bm),
                        2 => w.exti2().bits(port_bm),
                        3 => w.exti3().bits(port_bm),
                        _ => w,
                    });
                }
                4 | 5 | 6 | 7 => {
                    // no need to assert that PH is not port,
                    // since line is assert on port above
                    syscfg.exticr2.modify(|_, w| match line {
                        4 => w.exti4().bits(port_bm),
                        5 => w.exti5().bits(port_bm),
                        6 => w.exti6().bits(port_bm),
                        7 => w.exti7().bits(port_bm),
                        _ => w,
                    });
                }
                8 | 9 | 10 | 11 => {
                    syscfg.exticr3.modify(|_, w| match line {
                        8 => w.exti8().bits(port_bm),
                        9 => w.exti9().bits(port_bm),
                        10 => w.exti10().bits(port_bm),
                        11 => w.exti11().bits(port_bm),
                        _ => w,
                    });
                }
                12 | 13 | 14 | 15 => {
                    syscfg.exticr4.modify(|_, w| match line {
                        12 => w.exti12().bits(port_bm),
                        13 => w.exti13().bits(port_bm),
                        14 => w.exti14().bits(port_bm),
                        15 => w.exti15().bits(port_bm),
                        _ => w,
                    });
                }
                _ => (),
            };
        }

        let bm: u32 = 1 << line;

        unsafe {
            match edge {
                TriggerEdge::Rising => self.raw.rtsr.modify(|r, w| w.bits(r.bits() | bm)),
                TriggerEdge::Falling => self.raw.ftsr.modify(|r, w| w.bits(r.bits() | bm)),
                TriggerEdge::Both => {
                    self.raw.rtsr.modify(|r, w| w.bits(r.bits() | bm));
                    self.raw.ftsr.modify(|r, w| w.bits(r.bits() | bm));
                }
            }

            self.raw.imr.modify(|r, w| w.bits(r.bits() | bm));
        }
    }

    /// Starts listening on a configurable interrupt line.
    ///
    /// The edges that should trigger the interrupt can be configured with
    /// `edge`.
    #[inline]
    pub fn listen_configurable(&mut self, line: u8, edge: TriggerEdge) {
        let bm: u32 = 1 << line;

        unsafe {
            match edge {
                TriggerEdge::Rising => self.raw.rtsr.modify(|r, w| w.bits(r.bits() | bm)),
                TriggerEdge::Falling => self.raw.ftsr.modify(|r, w| w.bits(r.bits() | bm)),
                TriggerEdge::Both => {
                    self.raw.rtsr.modify(|r, w| w.bits(r.bits() | bm));
                    self.raw.ftsr.modify(|r, w| w.bits(r.bits() | bm));
                }
            }

            self.raw.imr.modify(|r, w| w.bits(r.bits() | bm));
        }
    }

    /// Disables the interrupt on `line`.
    pub fn unlisten(&mut self, line: u8) {
        let bm = 1 << line;

        // Safety: We clear the correct bit and have unique ownership of the EXTI registers here.
        unsafe {
            self.raw.imr.modify(|r, w| w.bits(r.bits() & !bm));
            self.raw.rtsr.modify(|r, w| w.bits(r.bits() & !bm));
            self.raw.ftsr.modify(|r, w| w.bits(r.bits() & !bm));
        }
    }

    /// Marks `line` as "pending".
    ///
    /// This will cause an interrupt if the EXTI was previously configured to
    /// listen on `line`.
    ///
    /// If `line` is already pending, this does nothing.
    pub fn pend(line: u8) {
        // Safety:
        // - We've ensured that the only 1-bit written is a valid line.
        // - This mirrors the `NVIC::pend` API and implementation, which is
        //   presumed safe.
        // - This is a "set by writing 1" register (ie. writing 0 does nothing),
        //   and this is a single write operation that cannot be interrupted.
        unsafe {
            (*EXTI::ptr()).swier.write(|w| w.bits(1 << line));
        }
    }

    /// Marks `line` as "not pending".
    ///
    /// This should be called from an interrupt handler to ensure that the
    /// interrupt doesn't continuously fire.
    pub fn unpend(line: u8) {
        // Safety:
        // - We've ensured that the only 1-bit written is a valid line.
        // - This mirrors the `NVIC::pend` API and implementation, which is
        //   presumed safe.
        // - This is a "clear by writing 1" register, and this is a single write
        //   operation that cannot be interrupted.
        unsafe {
            (*EXTI::ptr()).pr.write(|w| w.bits(1 << line));
        }
    }

    /// Returns whether `line` is currently marked as pending.
    pub fn is_pending(line: u8) -> bool {
        let bm: u32 = 1 << line;

        // Safety: This is a read without side effects that cannot be
        // interrupted.
        let pr = unsafe { (*EXTI::ptr()).pr.read().bits() };

        pr & bm != 0
    }
}

// //! External interrupt controller
// use crate::bb;
// use crate::stm32::EXTI;

// pub enum TriggerEdge {
//     Rising,
//     Falling,
//     All,
// }

// pub trait ExtiExt {
//     fn listen(&self, line: u8, edge: TriggerEdge);
//     fn unlisten(&self, line: u8);
//     fn pend_interrupt(&self, line: u8);
//     fn clear_irq(&self, line: u8);
// }

// impl ExtiExt for EXTI {
//     fn listen(&self, line: u8, edge: TriggerEdge) {
//         assert!(line < 24);
//         match edge {
//             TriggerEdge::Rising => bb::set(&self.rtsr, line),
//             TriggerEdge::Falling => bb::set(&self.ftsr, line),
//             TriggerEdge::All => {
//                 bb::set(&self.rtsr, line);
//                 bb::set(&self.ftsr, line);
//             }
//         }
//         bb::set(&self.imr, line);
//     }

//     fn unlisten(&self, line: u8) {
//         assert!(line < 24);
//         bb::clear(&self.rtsr, line);
//         bb::clear(&self.ftsr, line);
//         bb::clear(&self.imr, line);
//     }

//     fn pend_interrupt(&self, line: u8) {
//         assert!(line < 24);
//         bb::set(&self.swier, line);
//     }

//     fn clear_irq(&self, line: u8) {
//         assert!(line < 24);
//         bb::set(&self.pr, line);
//     }
// }
