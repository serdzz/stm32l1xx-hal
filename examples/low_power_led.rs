#![deny(warnings)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate stm32l1xx_hal as hal;

use core::panic::PanicInfo;
use embedded_hal::digital::v2::OutputPin;
use hal::prelude::*;
use hal::pwr::StopModeConfig;
use hal::rcc::{Config, PLLDiv, PLLMul, PLLSource};
use hal::rtc::{Event, Rtc};
use hal::stm32;
use rt::entry;

// Magic number to detect if RTC was initialized
const MAGIC_NUMBER: u32 = 0x32F2;
// Wakeup counter register index
const WAKEUP_COUNTER_REG: usize = 1;
// RTC wakeup interval in seconds
const WAKEUP_INTERVAL: u32 = 3;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();
    let mut pwr = dp.PWR;
    let mut exti = dp.EXTI;

    // Configure GPIO for LED (PB7 based on blinky example)
    let gpiob = dp.GPIOB.split();
    let mut led = gpiob.pb7.into_push_pull_output();

    // Blink LED rapidly at startup to show device is running
    for _ in 0..5 {
        led.set_high().unwrap();
        cortex_m::asm::delay(1_000_000);
        led.set_low().unwrap();
        cortex_m::asm::delay(1_000_000);
    }

    // Configure system clock to use HSE at 24MHz with PLL
    let rcc_config = Config::pll(PLLSource::HSE(24.mhz()), PLLMul::Mul4, PLLDiv::Div4);
    let mut rcc = dp.RCC.freeze(rcc_config);

    // Initialize RTC with LSE
    let mut rtc = Rtc::new(dp.RTC, &mut pwr);

    // Check if this is first boot after power loss
    if rtc.is_initialized(0, MAGIC_NUMBER) {
        let wakeup_count = rtc.read_backup_register(WAKEUP_COUNTER_REG);
        rtc.write_backup_register(WAKEUP_COUNTER_REG, wakeup_count + 1);

        // Blink LED to show wakeup count (slow blinks)
        for _ in 0..wakeup_count.min(10) {
            led.set_high().unwrap();
            cortex_m::asm::delay(500_000);
            led.set_low().unwrap();
            cortex_m::asm::delay(500_000);
        }
    } else {
        // First boot - initialize RTC
        use time::{Date, Month, PrimitiveDateTime, Time};
        let datetime = PrimitiveDateTime::new(
            Date::from_calendar_date(2025, Month::November, 26).unwrap(),
            Time::from_hms(21, 0, 0).unwrap(),
        );

        rtc.set_datetime(&datetime).unwrap();
        rtc.mark_initialized(0, MAGIC_NUMBER);
        rtc.write_backup_register(WAKEUP_COUNTER_REG, 0);

        // Very rapid blinks to show first boot
        for _ in 0..10 {
            led.set_high().unwrap();
            cortex_m::asm::delay(200_000);
            led.set_low().unwrap();
            cortex_m::asm::delay(200_000);
        }
    }

    // Configure RTC wakeup timer
    rtc.enable_wakeup(WAKEUP_INTERVAL);
    rtc.listen(&mut exti, Event::Wakeup);

    let mut scb = cp.SCB;
    let mut pwr_control = pwr.constrain();

    loop {
        // Clear any pending wakeup interrupt
        rtc.unpend(Event::Wakeup);

        // Turn off LED before sleep
        led.set_low().unwrap();

        // Configure for STOP mode with ultra-low-power
        let stop_config = StopModeConfig::ultra_low_power();
        pwr_control.stop_mode(stop_config, &mut scb);

        // Wait for interrupt (WFI) - enters STOP mode
        cortex_m::asm::wfi();

        // After wakeup from STOP mode - reconfigure clocks
        rcc.reconfigure_after_stop();

        // Blink LED once to show wakeup
        led.set_high().unwrap();
        cortex_m::asm::delay(2_000_000);
        led.set_low().unwrap();
        cortex_m::asm::delay(1_000_000);
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
