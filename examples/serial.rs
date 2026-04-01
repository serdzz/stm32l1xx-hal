//! # Serial example with runtime baud rate change
//!
//! This example demonstrates changing the baud rate of USART3 at runtime
//! using the `reconfigure` method.
//!
//! Hardware connections (STM32L152C-DISCO or compatible):
//!   PB10 — USART3 TX
//!   PB11 — USART3 RX
//!
//! Sequence:
//!   1. Start at 9600 bps — sends a greeting, waits for any byte
//!   2. Switch to 115200 bps
//!   3. Confirm the new speed over the new rate, then echo forever
//!
//! Connect a USB-UART adapter and observe the speed change.

#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate nb;
extern crate panic_semihosting;
extern crate stm32l1xx_hal as hal;

use core::fmt::Write;
use hal::prelude::*;
use hal::rcc::Config;
use hal::serial;
use hal::serial::SerialExt;
use hal::stm32;
use nb::block;
use rt::entry;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().unwrap();

    let mut rcc = dp.RCC.freeze(Config::hsi());

    let gpiob = dp.GPIOB.split();
    let tx = gpiob.pb10;
    let rx = gpiob.pb11;

    // --- Phase 1: communicate at 9600 bps ---
    let mut serial = dp
        .USART3
        .usart(
            (tx, rx),
            serial::Config::default().baudrate(9_600_u32.bps()),
            &mut rcc,
        )
        .unwrap();

    serial.write_str("Hello at 9600 bps!\r\n").unwrap();
    serial
        .write_str("Send any byte to switch to 115200 bps...\r\n")
        .unwrap();

    // Wait for a trigger byte from the host before switching
    block!(serial.read()).ok();

    // --- Phase 2: switch to 115200 bps at runtime ---
    //
    // `reconfigure` waits for the current TX to finish, briefly disables
    // USART, writes the new BRR value, then re-enables USART.
    serial.reconfigure(serial::Config::default().baudrate(115_200_u32.bps()), &rcc);

    // Now we are running at 115200 — host must switch its terminal speed too
    serial
        .write_str("Now running at 115200 bps — echo mode active\r\n")
        .unwrap();

    // --- Phase 3: echo loop at 115200 bps ---
    let (mut tx, mut rx) = serial.split();

    loop {
        let received = block!(rx.read()).unwrap();
        // Echo back with a newline prefix so output is clearly visible
        tx.write_str("\r\nrx: ").unwrap();
        block!(tx.write(received)).ok();
    }
}
