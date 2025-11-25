#![deny(warnings)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate cortex_m_semihosting as sh;
extern crate panic_semihosting;
extern crate stm32l1xx_hal as hal;

use core::cell::RefCell;
use core::ops::DerefMut;

use cortex_m::interrupt::Mutex;
use hal::exti::TriggerEdge;
use hal::prelude::*;
use hal::stm32::{self, interrupt, Interrupt};
use rt::entry;
use sh::hprintln;

// Используем RefCell для хранения пина в статической переменной
type ButtonPin = hal::gpio::gpioa::PA0<hal::gpio::Input<hal::gpio::Floating>>;
static BUTTON: Mutex<RefCell<Option<ButtonPin>>> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().unwrap();
    let mut cp = cortex_m::Peripherals::take().unwrap();

    // Получаем GPIO и SYSCFG
    let gpioa = dp.GPIOA.split();
    let mut syscfg = dp.SYSCFG;
    let mut exti = dp.EXTI;

    // Настраиваем PA0 как input с прерыванием
    let mut button = gpioa.pa0.into_floating_input();

    // Настраиваем прерывание на кнопке
    button.make_interrupt_source(&mut syscfg);
    button.enable_interrupt(&mut exti);
    button.trigger_on_edge(&mut exti, TriggerEdge::Falling);

    // Сохраняем пин в глобальной переменной
    cortex_m::interrupt::free(move |cs| {
        *BUTTON.borrow(cs).borrow_mut() = Some(button);
    });

    // Включаем прерывание в NVIC
    unsafe {
        cp.NVIC.set_priority(Interrupt::EXTI0, 1);
        cortex_m::peripheral::NVIC::unmask(Interrupt::EXTI0);
    }

    hprintln!("Waiting for button press on PA0...");

    loop {
        cortex_m::asm::wfi(); // Wait for interrupt
    }
}

#[interrupt]
fn EXTI0() {
    static mut COUNT: i32 = 0;

    *COUNT += 1;
    hprintln!("Button pressed! Count: {}", COUNT);

    cortex_m::interrupt::free(|cs| {
        if let Some(ref mut button) = *BUTTON.borrow(cs).borrow_mut().deref_mut() {
            // Очищаем флаг прерывания напрямую на пине!
            button.clear_interrupt_pending_bit();
        }
    });
}
