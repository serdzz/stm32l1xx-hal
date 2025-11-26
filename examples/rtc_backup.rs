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
use time::PrimitiveDateTime;

// Magic number for RTC initialization check
const RTC_MAGIC_NUMBER: u32 = 0x32F2;
const RTC_BACKUP_REG: usize = 0;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().unwrap();
    let mut pwr = dp.PWR;
    let mut rcc = dp.RCC.freeze(Config::hsi());

    // Initialize RTC with LSE (external 32.768 kHz crystal)
    let mut rtc = Rtc::new(dp.RTC, &mut pwr);

    // Check if RTC was previously initialized
    if !rtc.is_initialized(RTC_BACKUP_REG, RTC_MAGIC_NUMBER) {
        hprintln!("RTC not initialized - setting time");

        // Set initial date and time: 2025-01-01 12:00:00
        let date = time::Date::from_calendar_date(2025, time::Month::January, 1).unwrap();
        let time = time::Time::from_hms(12, 0, 0).unwrap();
        let datetime = PrimitiveDateTime::new(date, time);

        // Set the date and time
        rtc.set_datetime(&datetime).unwrap();

        // Mark RTC as initialized by writing magic number
        rtc.mark_initialized(RTC_BACKUP_REG, RTC_MAGIC_NUMBER);

        hprintln!("RTC initialized and marked");
    } else {
        hprintln!("RTC already initialized - keeping current time");
    }

    // Read and display current time
    let datetime = rtc.get_datetime();
    hprintln!(
        "Current RTC time: {:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        datetime.year(),
        u8::from(datetime.month()),
        datetime.day(),
        datetime.hour(),
        datetime.minute(),
        datetime.second()
    );

    // Demonstrate backup register usage
    // Write counter to backup register 1
    let counter = rtc.read_backup_register(1);
    hprintln!("Boot counter: {}", counter);
    rtc.write_backup_register(1, counter + 1);

    // Enable RTC wakeup timer (1 second interval)
    rtc.enable_wakeup(1);
    hprintln!("RTC wakeup timer enabled");

    loop {
        // Wait for wakeup event
        cortex_m::asm::wfi();

        // Read and display time every wakeup
        let datetime = rtc.get_datetime();
        hprintln!(
            "{:02}:{:02}:{:02}",
            datetime.hour(),
            datetime.minute(),
            datetime.second()
        );

        // Clear wakeup flag
        rtc.unpend(hal::rtc::Event::Wakeup);
    }
}
