#![deny(warnings)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate cortex_m_semihosting as sh;
extern crate panic_semihosting;
extern crate stm32l1xx_hal as hal;

use hal::prelude::*;
use hal::pwr::StopModeConfig;
use hal::rcc::{Config, PLLDiv, PLLMul, PLLSource, SysClkSource};
use hal::rtc::{Event, Rtc};
use hal::stm32;
use rt::entry;
use sh::hprintln;

// Magic number to detect if RTC was initialized
const MAGIC_NUMBER: u32 = 0x32F2;
// Wakeup counter register index
const WAKEUP_COUNTER_REG: usize = 1;
// RTC wakeup interval in seconds
const WAKEUP_INTERVAL: u32 = 5;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();
    let mut pwr = dp.PWR;
    let mut exti = dp.EXTI;

    // Configure system clock to use HSE at 24MHz with PLL
    // HSE 24MHz -> PLL x4 / 4 = 24MHz system clock
    let rcc_config = Config::pll(PLLSource::HSE(24.mhz()), PLLMul::Mul4, PLLDiv::Div4);
    let mut rcc = dp.RCC.freeze(rcc_config);

    hprintln!("=== STM32L1 Advanced Low Power Example ===");
    hprintln!("System clock configured to HSE 24MHz with PLL");

    // Verify clock source
    let clk_src = rcc.get_sysclk_source();
    hprintln!("Current system clock source: {:?}", clk_src);

    // Initialize RTC with LSE (Low Speed External oscillator)
    let mut rtc = Rtc::new(dp.RTC, &mut pwr);

    // Check if this is first boot after power loss
    if rtc.is_initialized(0, MAGIC_NUMBER) {
        let wakeup_count = rtc.read_backup_register(WAKEUP_COUNTER_REG);
        hprintln!("Woke up from deep sleep! Wakeup count: {}", wakeup_count);
        rtc.write_backup_register(WAKEUP_COUNTER_REG, wakeup_count + 1);
    } else {
        hprintln!("First boot - initializing RTC...");

        // Set initial date and time (2025-11-26 21:00:00)
        use time::{Date, Month, PrimitiveDateTime, Time};
        let datetime = PrimitiveDateTime::new(
            Date::from_calendar_date(2025, Month::November, 26).unwrap(),
            Time::from_hms(21, 0, 0).unwrap(),
        );

        rtc.set_datetime(&datetime).unwrap();
        rtc.mark_initialized(0, MAGIC_NUMBER);
        rtc.write_backup_register(WAKEUP_COUNTER_REG, 0);

        hprintln!("RTC initialized to: 2025-11-26 21:00:00");
    }

    // Display current RTC time
    let current_time = rtc.get_datetime();
    hprintln!(
        "Current RTC time: {:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        current_time.year(),
        u8::from(current_time.month()),
        current_time.day(),
        current_time.hour(),
        current_time.minute(),
        current_time.second()
    );

    // Configure RTC wakeup timer
    hprintln!(
        "Configuring RTC wakeup timer for {} seconds...",
        WAKEUP_INTERVAL
    );
    rtc.enable_wakeup(WAKEUP_INTERVAL);
    rtc.listen(&mut exti, Event::Wakeup);

    hprintln!("Entering deep sleep (STOP mode)...");
    hprintln!("System will wake up every {} seconds", WAKEUP_INTERVAL);

    let mut scb = cp.SCB;
    let mut wakeup_count: u32 = 0;

    // Create PWR wrapper for STOP mode configuration
    let mut pwr_control = pwr.constrain();

    loop {
        // Clear any pending wakeup interrupt
        rtc.unpend(Event::Wakeup);

        hprintln!("\n--- Cycle {} ---", wakeup_count);
        hprintln!(
            "Clock before STOP: {:?} ({} MHz)",
            rcc.get_sysclk_source(),
            rcc.clocks.sys_clk().0 / 1_000_000
        );

        // Configure for STOP mode with ultra-low-power
        let stop_config = StopModeConfig::ultra_low_power();
        pwr_control.stop_mode(stop_config, &mut scb);

        hprintln!("Entering STOP mode...");

        // Wait for interrupt (WFI) - enters STOP mode
        cortex_m::asm::wfi();

        // After wakeup from STOP mode
        hprintln!("Woke up!");
        hprintln!(
            "Clock after STOP (before reconfig): {:?}",
            rcc.get_sysclk_source()
        );

        // Reconfigure clocks back to HSE/PLL
        rcc.reconfigure_after_stop();

        hprintln!(
            "Clock after reconfiguration: {:?} ({} MHz)",
            rcc.get_sysclk_source(),
            rcc.clocks.sys_clk().0 / 1_000_000
        );

        // Verify we're back on PLL
        if rcc.get_sysclk_source() == SysClkSource::PLL {
            hprintln!("✓ Successfully reconfigured to PLL");
        } else {
            hprintln!("✗ Clock reconfiguration failed!");
        }

        wakeup_count += 1;
    }
}
