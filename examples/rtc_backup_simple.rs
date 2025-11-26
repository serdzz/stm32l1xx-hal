#![deny(warnings)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate cortex_m_semihosting as sh;
extern crate panic_semihosting;
extern crate stm32l1xx_hal as hal;

use hal::prelude::*;
use hal::rcc::Config;
use hal::rtc::Rtc;
use hal::stm32;
use rt::entry;
use sh::hprintln;

// Magic number to detect if RTC was initialized
const MAGIC_NUMBER: u32 = 0x32F2;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().unwrap();
    let mut pwr = dp.PWR;
    let _rcc = dp.RCC.freeze(Config::hsi());

    // Initialize RTC
    let mut rtc = Rtc::new(dp.RTC, &mut pwr);

    // Check if this is first boot after power loss
    if rtc.is_initialized(0, MAGIC_NUMBER) {
        hprintln!("RTC backup domain powered - data preserved!");

        // Read boot counter from backup register
        let boot_count = rtc.read_backup_register(1);
        hprintln!("Boot count: {}", boot_count);

        // Increment and save
        rtc.write_backup_register(1, boot_count + 1);
    } else {
        hprintln!("First boot or VBAT power lost - initializing...");

        // Mark as initialized
        rtc.mark_initialized(0, MAGIC_NUMBER);

        // Initialize boot counter
        rtc.write_backup_register(1, 1);
    }

    // Demonstrate using multiple backup registers
    rtc.write_backup_register(2, 0xDEADBEEF);
    rtc.write_backup_register(3, 0xCAFEBABE);

    let val2 = rtc.read_backup_register(2);
    let val3 = rtc.read_backup_register(3);

    hprintln!("Backup register 2: 0x{:08X}", val2);
    hprintln!("Backup register 3: 0x{:08X}", val3);

    hprintln!("Done! These values will persist across resets if VBAT is present.");

    loop {
        cortex_m::asm::wfi();
    }
}
