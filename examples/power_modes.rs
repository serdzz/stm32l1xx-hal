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
use hal::rcc::Config;
use hal::rtc::{Event, Rtc};
use hal::stm32;
use rt::entry;
use sh::hprintln;

// Magic number to detect if RTC was initialized
const MAGIC_NUMBER: u32 = 0x32F2;
const MODE_COUNTER_REG: usize = 1;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();
    let mut pwr_raw = dp.PWR;
    let mut exti = dp.EXTI;

    let rcc_config = Config::hsi();
    let mut rcc = dp.RCC.freeze(rcc_config);

    hprintln!("=== STM32L1 Power Modes Example ===");

    // Initialize RTC with LSE
    let mut rtc = Rtc::new(dp.RTC, &mut pwr_raw);

    // Check if this is first boot
    if rtc.is_initialized(0, MAGIC_NUMBER) {
        let mode_counter = rtc.read_backup_register(MODE_COUNTER_REG);
        hprintln!("Resumed from low power mode. Counter: {}", mode_counter);
        rtc.write_backup_register(MODE_COUNTER_REG, mode_counter + 1);
    } else {
        hprintln!("First boot - initializing...");
        rtc.mark_initialized(0, MAGIC_NUMBER);
        rtc.write_backup_register(MODE_COUNTER_REG, 0);
    }

    // Configure RTC wakeup for 3 seconds
    rtc.enable_wakeup(3);
    rtc.listen(&mut exti, Event::Wakeup);

    let mut scb = cp.SCB;
    let mut pwr = pwr_raw.constrain();

    hprintln!("\n=== Testing different power modes ===\n");

    let mode_counter = rtc.read_backup_register(MODE_COUNTER_REG);
    let mode = mode_counter % 4;

    match mode {
        0 => {
            hprintln!("Mode 0: STOP with Main Voltage Regulator");
            hprintln!("- Faster wakeup");
            hprintln!("- Higher power consumption");
            rtc.unpend(Event::Wakeup);

            let config = StopModeConfig::main_mode();
            pwr.stop_mode(config, &mut scb);
            cortex_m::asm::wfi();

            // Reconfigure clocks after wakeup
            rcc.reconfigure_after_stop();
            hprintln!("Woke up from STOP (main regulator)!");
        }

        1 => {
            hprintln!("Mode 1: STOP with Low-Power Voltage Regulator");
            hprintln!("- Slower wakeup than main mode");
            hprintln!("- Lower power consumption");
            rtc.unpend(Event::Wakeup);

            let config = StopModeConfig::low_power();
            pwr.stop_mode(config, &mut scb);
            cortex_m::asm::wfi();

            rcc.reconfigure_after_stop();
            hprintln!("Woke up from STOP (low-power regulator)!");
        }

        2 => {
            hprintln!("Mode 2: STOP with Ultra-Low-Power");
            hprintln!("- Slowest wakeup");
            hprintln!("- Lowest power consumption in STOP");
            hprintln!("- Voltage reference and temp sensor disabled");
            rtc.unpend(Event::Wakeup);

            let config = StopModeConfig::ultra_low_power();
            pwr.stop_mode(config, &mut scb);
            cortex_m::asm::wfi();

            rcc.reconfigure_after_stop();
            hprintln!("Woke up from STOP (ultra-low-power)!");
        }

        3 => {
            hprintln!("Mode 3: SLEEP mode");
            hprintln!("- CPU stopped, peripherals running");
            hprintln!("- Fastest wakeup");
            hprintln!("- Higher power consumption than STOP");
            rtc.unpend(Event::Wakeup);

            pwr.sleep_mode(&mut scb);
            cortex_m::asm::wfi();

            hprintln!("Woke up from SLEEP!");
        }

        _ => unreachable!(),
    }

    hprintln!("\nCycle complete. Will test next mode on next wakeup.");
    hprintln!("Next mode will be: {}", (mode + 1) % 4);

    // Loop to prevent exit
    loop {
        cortex_m::asm::wfi();
    }
}
