use crate::bb;
use crate::stm32::rtc::{dr, tr};
use crate::stm32::{rcc::RegisterBlock, EXTI, PWR, RCC, RTC};
use cast::u16;
use core::convert::TryInto;
use core::fmt;
use core::marker::PhantomData;
use time::{Date, PrimitiveDateTime, Time};

/// Invalid input error
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Error {
    InvalidInputData,
}

pub enum Event {
    AlarmA,
    AlarmB,
    Wakeup,
    Timestamp,
}

/// RTC clock source LSE oscillator clock (type state)
pub struct Lse;
/// RTC clock source LSI oscillator clock (type state)
pub struct Lsi;

/// Real Time Clock peripheral
pub struct Rtc<CS = Lse> {
    /// RTC Peripheral register
    pub regs: RTC,
    _clock_source: PhantomData<CS>,
}

#[cfg(feature = "defmt")]
impl defmt::Format for Rtc {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "Rtc");
    }
}

impl fmt::Debug for Rtc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Rtc")
    }
}

/// LSE clock mode.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LSEClockMode {
    /// Enable LSE oscillator to use external crystal or ceramic resonator.
    Oscillator,
    /// Bypass LSE oscillator to use external clock source.
    /// Use this if an external oscillator is used which is not connected to `OSC32_IN` such as a MEMS resonator.
    Bypass,
}

impl Rtc<Lse> {
    /// Create and enable a new RTC with external crystal or ceramic resonator and default prescalers.
    pub fn new(regs: RTC, pwr: &mut PWR) -> Self {
        Self::with_config(regs, pwr, LSEClockMode::Oscillator, 255, 127)
    }

    /// Create and enable a new RTC, and configure its clock source and prescalers.
    ///
    /// From AN3371, Table 3, when using the LSE,
    /// set `prediv_s` to 255, and `prediv_a` to 127 to get a calendar clock of 1Hz.
    pub fn with_config(
        regs: RTC,
        pwr: &mut PWR,
        mode: LSEClockMode,
        prediv_s: u16,
        prediv_a: u8,
    ) -> Self {
        let mut result = Self {
            regs,
            _clock_source: PhantomData,
        };

        // Steps:
        // Enable PWR and DBP
        // Enable LSE (if needed)
        // Enable RTC Clock
        // Disable Write Protect
        // Enter Init
        // Configure 24 hour format
        // Set prescalers
        // Exit Init
        // Enable write protect

        unsafe {
            let rcc = &(*RCC::ptr());
            // As per the sample code, unlock comes first. (Enable PWR and DBP)
            result.unlock(rcc, pwr);
            // If necessary, enable the LSE.
            if rcc.csr.read().lserdy().bit_is_clear() {
                result.enable_lse(rcc, mode);
            }
            // Set clock source to LSE.
            rcc.csr.modify(|_, w| w.rtcsel().bits(1));
            result.enable(rcc);
        }

        result.modify(|regs| {
            // Set 24 Hour
            regs.cr.modify(|_, w| w.fmt().clear_bit());
            // Set prescalers
            regs.prer.modify(|_, w| unsafe {
                w.prediv_s().bits(prediv_s);
                w.prediv_a().bits(prediv_a)
            })
        });

        result
    }

    /// Enable the low frequency external oscillator. This is the only mode currently
    /// supported, to avoid exposing the `CR` and `CRS` registers.
    fn enable_lse(&mut self, rcc: &RegisterBlock, mode: LSEClockMode) {
        // Force a reset of the backup domain.
        self.backup_reset(rcc);
        // Enable the LSE.
        // Set CSR - Bit 8 (LSEON)
        bb::set(&rcc.csr, 8);
        match mode {
            // Set CSR - Bit 10 (LSEBYP)
            LSEClockMode::Bypass => bb::set(&rcc.csr, 10),
            // Clear CSR - Bit 10 (LSEBYP)
            LSEClockMode::Oscillator => bb::clear(&rcc.csr, 10),
        }
        while rcc.csr.read().lserdy().bit_is_clear() {}
    }
}

impl Rtc<Lsi> {
    /// Create and enable a new RTC with internal crystal and default prescalers.
    pub fn new_lsi(regs: RTC, pwr: &mut PWR) -> Self {
        Self::lsi_with_config(regs, pwr, 249, 127)
    }

    /// Create and enable a new RTC, and configure its clock source and prescalers.
    ///
    /// From AN3371, Table 3, when using the LSI,
    /// set `prediv_s` to 249, and `prediv_a` to 127 to get a calendar clock of 1Hz.
    pub fn lsi_with_config(regs: RTC, pwr: &mut PWR, prediv_s: u16, prediv_a: u8) -> Self {
        let mut result = Self {
            regs,
            _clock_source: PhantomData,
        };

        // Steps:
        // Enable PWR and DBP
        // Enable LSI (if needed)
        // Enable RTC Clock
        // Disable Write Protect
        // Enter Init
        // Configure 24 hour format
        // Set prescalers
        // Exit Init
        // Enable write protect

        unsafe {
            let rcc = &(*RCC::ptr());
            // As per the sample code, unlock comes first. (Enable PWR and DBP)
            result.unlock(rcc, pwr);
            // If necessary, enable the LSE.
            if rcc.csr.read().lsirdy().bit_is_clear() {
                result.enable_lsi(rcc);
            }
            // Set clock source to LSI.
            rcc.csr.modify(|_, w| w.rtcsel().bits(2));
            result.enable(rcc);
        }

        result.modify(|regs| {
            // Set 24 Hour
            regs.cr.modify(|_, w| w.fmt().clear_bit());
            // Set prescalers
            regs.prer.modify(|_, w| unsafe {
                w.prediv_s().bits(prediv_s);
                w.prediv_a().bits(prediv_a)
            })
        });

        result
    }

    fn enable_lsi(&mut self, rcc: &RegisterBlock) {
        // Force a reset of the backup domain.
        self.backup_reset(rcc);
        // Enable the LSI.
        rcc.csr.modify(|_, w| w.lsion().set_bit());
        while rcc.csr.read().lsirdy().bit_is_clear() {}
    }
}

impl<CS> Rtc<CS> {
    fn unlock(&mut self, rcc: &RegisterBlock, pwr: &mut PWR) {
        // Enable the backup interface
        rcc.apb1enr.write(|w| w.pwren().set_bit());

        pwr.cr.modify(|_, w| {
            w
                // Enable access to the backup registers
                .dbp()
                .set_bit()
        });
    }

    fn backup_reset(&mut self, rcc: &RegisterBlock) {
        // Set CSR - Bit 16 (BDRST)
        bb::set(&rcc.csr, 16);
        // Clear CSR - Bit 16 (BDRST)
        bb::clear(&rcc.csr, 16);
    }

    fn enable(&mut self, rcc: &RegisterBlock) {
        // Start the actual RTC.
        // Set CSR - Bit 22 (RTCEN)
        bb::set(&rcc.csr, 22);
    }

    pub fn set_prescalers(&mut self, prediv_s: u16, prediv_a: u8) {
        self.modify(|regs| {
            // Set prescalers
            regs.prer.modify(|_, w| unsafe {
                w.prediv_s().bits(prediv_s);
                w.prediv_a().bits(prediv_a)
            })
        });
    }

    pub fn enable_wakeup(&mut self, interval: u32) {
        self.regs.wpr.write(|w| unsafe { w.bits(0xCA) });
        self.regs.wpr.write(|w| unsafe { w.bits(0x53) });
        self.regs.cr.modify(|_, w| w.wute().clear_bit());
        self.regs.isr.modify(|_, w| w.wutf().clear_bit());
        while self.regs.isr.read().wutwf().bit_is_clear() {}

        if interval > 1 << 16 {
            self.regs
                .cr
                .modify(|_, w| unsafe { w.wucksel().bits(0b110) });
            let interval = u16(interval - (1 << 16) - 1)
                .expect("Interval was too large for wakeup timer");
            self.regs.wutr.write(|w| unsafe { w.wut().bits(interval) } );
        } else {
            self.regs
                .cr
                .modify(|_, w| unsafe { w.wucksel().bits(0b100) });
            let interval = u16(interval - 1)
                .expect("Interval was too large for wakeup timer");
            self.regs.wutr.write(|w| unsafe {w.wut().bits(interval)});
        }

        self.regs.cr.modify(|_, w| w.wute().set_bit());
        self.regs.wpr.write(|w| unsafe { w.bits(0xFF) });
    }


    /// Start listening for `event`
    pub fn listen(&mut self, exti: &mut EXTI, event: Event) {
        self.regs.wpr.write(|w| unsafe { w.bits(0xCA) });
        self.regs.wpr.write(|w| unsafe { w.bits(0x53) });
        match event {
            Event::AlarmA => {
                bb::set(&exti.rtsr, 17);
                bb::set(&exti.imr, 17);
                self.regs.cr.modify(|_, w| w.alraie().set_bit());
            }
            Event::AlarmB => {
                bb::set(&exti.rtsr, 17);
                bb::set(&exti.imr, 17);
                self.regs.cr.modify(|_, w| w.alrbie().set_bit());
            }
            Event::Wakeup => {
                bb::set(&exti.rtsr, 20);
                bb::set(&exti.imr, 20);
                self.regs.cr.modify(|_, w| w.wutie().set_bit());
            }
            Event::Timestamp => {
                bb::set(&exti.rtsr, 19);
                bb::set(&exti.imr, 19);
                self.regs.cr.modify(|_, w| w.tsie().set_bit());
            }
        }
        self.regs.wpr.write(|w| unsafe { w.bits(0xFF) });
    }

    pub fn disable_wakeup(&mut self) {
        self.regs.wpr.write(|w| unsafe { w.bits(0xCA) });
        self.regs.wpr.write(|w| unsafe { w.bits(0x53) });
        self.regs.cr.modify(|_, w| w.wute().clear_bit());
        self.regs.isr.modify(|_, w| w.wutf().clear_bit());
        self.regs.wpr.write(|w| unsafe { w.bits(0xFF) });
    }

    pub fn unpend(&mut self, event: Event) {
        let pwr = unsafe { &(*PWR::ptr()) };
        let exti = unsafe { &(*EXTI::ptr()) };
        pwr.cr.modify(|_,w| w.cwuf().set_bit());
        match event {
            Event::AlarmA => {
                self.regs.isr.modify(|_, w| w.alraf().clear_bit());
                bb::set(&exti.pr, 17);
            }
            Event::AlarmB => {
                self.regs.isr.modify(|_, w| w.alrbf().clear_bit());
                bb::set(&exti.pr, 17);
            }
            Event::Wakeup => {
                self.regs.isr.modify(|_, w| w.wutf().clear_bit());
                bb::set(&exti.pr, 20);
            }
            Event::Timestamp => {
                self.regs.isr.modify(|_, w| w.tsf().clear_bit());
                bb::set(&exti.pr, 19);
            }
        }
    }

    pub fn wakeup_timer(&mut self, val: u32) {
        self.regs.wpr.write(|w| unsafe { w.bits(0xCA) });
        self.regs.wpr.write(|w| unsafe { w.bits(0x53) });
        self.regs.isr.modify(|_, w| w.wutf().clear_bit());
        while self.regs.isr.read().wutf().bit_is_set(){}
        self.regs.wutr.write(|w| unsafe { w.wut().bits(val as u16) });
        self.regs.cr.write(|w| unsafe {
            w.wucksel().bits(0b100);
            w.wutie().set_bit();
            w.wute().set_bit()
        });
        self.regs.wpr.write(|w| unsafe { w.bits(0xFF) });
    }

    /// As described in Section 27.3.7 in RM0316,
    /// this function is used to disable write protection
    /// when modifying an RTC register
    fn modify<F>(&mut self, mut closure: F)
    where
        F: FnMut(&mut RTC),
    {
        // Disable write protection
        self.regs.wpr.write(|w| unsafe { w.bits(0xCA) });
        self.regs.wpr.write(|w| unsafe { w.bits(0x53) });
        // Enter init mode
        let isr = self.regs.isr.read();
        if isr.initf().bit_is_clear() {
            self.regs.isr.modify(|_, w| w.init().set_bit());
            while self.regs.isr.read().initf().bit_is_clear() {}
        }
        // Invoke closure
        closure(&mut self.regs);
        // Exit init mode
        self.regs.isr.modify(|_, w| w.init().clear_bit());
        // wait for last write to be done
        while !self.regs.isr.read().initf().bit_is_clear() {}

        // Enable write protection
        self.regs.wpr.write(|w| unsafe { w.bits(0xFF) });
    }

    /// Set the time using time::Time.
    pub fn set_time(&mut self, time: &Time) -> Result<(), Error> {
        let (ht, hu) = bcd2_encode(time.hour().into())?;
        let (mnt, mnu) = bcd2_encode(time.minute().into())?;
        let (st, su) = bcd2_encode(time.second().into())?;
        self.modify(|regs| {
            regs.tr.write(|w| unsafe {
                w.ht().bits(ht);
                w.hu().bits(hu);
                w.mnt().bits(mnt);
                w.mnu().bits(mnu);
                w.st().bits(st);
                w.su().bits(su);
                w.pm().clear_bit()
            })
        });

        Ok(())
    }

    /// Set the seconds [0-59].
    pub fn set_seconds(&mut self, seconds: u8) -> Result<(), Error> {
        if seconds > 59 {
            return Err(Error::InvalidInputData);
        }
        let (st, su) = bcd2_encode(seconds.into())?;
        self.modify(|regs| {
            regs.tr
                .modify(|_, w| unsafe { w.st().bits(st).su().bits(su) })
        });

        Ok(())
    }

    /// Set the minutes [0-59].
    pub fn set_minutes(&mut self, minutes: u8) -> Result<(), Error> {
        if minutes > 59 {
            return Err(Error::InvalidInputData);
        }
        let (mnt, mnu) = bcd2_encode(minutes.into())?;
        self.modify(|regs| {
            regs.tr
                .modify(|_, w| unsafe { w.mnt().bits(mnt).mnu().bits(mnu) })
        });

        Ok(())
    }

    /// Set the hours [0-23].
    pub fn set_hours(&mut self, hours: u8) -> Result<(), Error> {
        if hours > 23 {
            return Err(Error::InvalidInputData);
        }
        let (ht, hu) = bcd2_encode(hours.into())?;

        self.modify(|regs| {
            regs.tr
                .modify(|_, w| unsafe { w.ht().bits(ht).hu().bits(hu) })
        });

        Ok(())
    }

    /// Set the day of week [1-7].
    pub fn set_weekday(&mut self, weekday: u8) -> Result<(), Error> {
        if !(1..=7).contains(&weekday) {
            return Err(Error::InvalidInputData);
        }
        self.modify(|regs| regs.dr.modify(|_, w| unsafe { w.wdu().bits(weekday) }));

        Ok(())
    }

    /// Set the day of month [1-31].
    pub fn set_day(&mut self, day: u8) -> Result<(), Error> {
        if !(1..=31).contains(&day) {
            return Err(Error::InvalidInputData);
        }
        let (dt, du) = bcd2_encode(day as u32)?;
        self.modify(|regs| {
            regs.dr
                .modify(|_, w| unsafe { w.dt().bits(dt).du().bits(du) })
        });

        Ok(())
    }

    /// Set the month [1-12].
    pub fn set_month(&mut self, month: u8) -> Result<(), Error> {
        if !(1..=12).contains(&month) {
            return Err(Error::InvalidInputData);
        }
        let (mt, mu) = bcd2_encode(month as u32)?;
        self.modify(|regs| {
            regs.dr
                .modify(|_, w| unsafe { w.mt().bit(mt > 0).mu().bits(mu) })
        });

        Ok(())
    }

    /// Set the year [1970-2069].
    ///
    /// The year cannot be less than 1970, since the Unix epoch is assumed (1970-01-01 00:00:00).
    /// Also, the year cannot be greater than 2069 since the RTC range is 0 - 99.
    pub fn set_year(&mut self, year: u16) -> Result<(), Error> {
        if !(1970..=2069).contains(&year) {
            return Err(Error::InvalidInputData);
        }
        let (yt, yu) = bcd2_encode(year as u32 - 1970)?;
        self.modify(|regs| {
            regs.dr
                .modify(|_, w| unsafe { w.yt().bits(yt).yu().bits(yu) })
        });

        Ok(())
    }

    /// Set the date.
    ///
    /// The year cannot be less than 1970, since the Unix epoch is assumed (1970-01-01 00:00:00).
    /// Also, the year cannot be greater than 2069 since the RTC range is 0 - 99.
    pub fn set_date(&mut self, date: &Date) -> Result<(), Error> {
        if !(1970..=2069).contains(&date.year()) {
            return Err(Error::InvalidInputData);
        }

        let (yt, yu) = bcd2_encode((date.year() - 1970) as u32)?;
        let (mt, mu) = bcd2_encode(u8::from(date.month()).into())?;
        let (dt, du) = bcd2_encode(date.day().into())?;

        self.modify(|regs| {
            regs.dr.write(|w| unsafe {
                w.dt().bits(dt);
                w.du().bits(du);
                w.mt().bit(mt > 0);
                w.mu().bits(mu);
                w.yt().bits(yt);
                w.yu().bits(yu)
            })
        });

        Ok(())
    }

    /// Set the date and time.
    ///
    /// The year cannot be less than 1970, since the Unix epoch is assumed (1970-01-01 00:00:00).
    /// Also, the year cannot be greater than 2069 since the RTC range is 0 - 99.
    pub fn set_datetime(&mut self, date: &PrimitiveDateTime) -> Result<(), Error> {
        if !(1970..=2069).contains(&date.year()) {
            return Err(Error::InvalidInputData);
        }

        let (yt, yu) = bcd2_encode((date.year() - 1970) as u32)?;
        let (mt, mu) = bcd2_encode(u8::from(date.month()).into())?;
        let (dt, du) = bcd2_encode(date.day().into())?;

        let (ht, hu) = bcd2_encode(date.hour().into())?;
        let (mnt, mnu) = bcd2_encode(date.minute().into())?;
        let (st, su) = bcd2_encode(date.second().into())?;

        self.modify(|regs| {
            regs.dr.write(|w| unsafe {
                w.dt().bits(dt);
                w.du().bits(du);
                w.mt().bit(mt > 0);
                w.mu().bits(mu);
                w.yt().bits(yt);
                w.yu().bits(yu)
            });
            regs.tr.write(|w| unsafe {
                w.ht().bits(ht);
                w.hu().bits(hu);
                w.mnt().bits(mnt);
                w.mnu().bits(mnu);
                w.st().bits(st);
                w.su().bits(su);
                w.pm().clear_bit()
            })
        });

        Ok(())
    }

    pub fn get_datetime(&mut self) -> PrimitiveDateTime {
        // Wait for Registers synchronization flag,  to ensure consistency between the RTC_SSR, RTC_TR and RTC_DR shadow registers.
        while self.regs.isr.read().rsf().bit_is_clear() {}

        // Reading either RTC_SSR or RTC_TR locks the values in the higher-order calendar shadow registers until RTC_DR is read.
        // So it is important to always read SSR, TR and then DR or TR and then DR.
        let tr = self.regs.tr.read();
        let dr = self.regs.dr.read();
        // In case the software makes read accesses to the calendar in a time interval smaller
        // than 2 RTCCLK periods: RSF must be cleared by software after the first calendar read.
        self.regs.isr.modify(|_, w| w.rsf().clear_bit());

        let seconds = decode_seconds(&tr);
        let minutes = decode_minutes(&tr);
        let hours = decode_hours(&tr);
        let day = decode_day(&dr);
        let month = decode_month(&dr);
        let year = decode_year(&dr);

        PrimitiveDateTime::new(
            Date::from_calendar_date(year.into(), month.try_into().unwrap(), day).unwrap(),
            Time::from_hms(hours, minutes, seconds).unwrap(),
        )
    }
}

// Two 32-bit registers (RTC_TR and RTC_DR) contain the seconds, minutes, hours (12- or 24-hour format), day (day
// of week), date (day of month), month, and year, expressed in binary coded decimal format
// (BCD). The sub-seconds value is also available in binary format.
//
// The following helper functions encode into BCD format from integer and
// decode to an integer from a BCD value respectively.
fn bcd2_encode(word: u32) -> Result<(u8, u8), Error> {
    let l = match (word / 10).try_into() {
        Ok(v) => v,
        Err(_) => {
            return Err(Error::InvalidInputData);
        }
    };
    let r = match (word % 10).try_into() {
        Ok(v) => v,
        Err(_) => {
            return Err(Error::InvalidInputData);
        }
    };

    Ok((l, r))
}

fn bcd2_decode(fst: u8, snd: u8) -> u32 {
    (fst * 10 + snd).into()
}

#[inline(always)]
fn decode_seconds(tr: &tr::R) -> u8 {
    bcd2_decode(tr.st().bits(), tr.su().bits()) as u8
}

#[inline(always)]
fn decode_minutes(tr: &tr::R) -> u8 {
    bcd2_decode(tr.mnt().bits(), tr.mnu().bits()) as u8
}

#[inline(always)]
fn decode_hours(tr: &tr::R) -> u8 {
    bcd2_decode(tr.ht().bits(), tr.hu().bits()) as u8
}

#[inline(always)]
fn decode_day(dr: &dr::R) -> u8 {
    bcd2_decode(dr.dt().bits(), dr.du().bits()) as u8
}

#[inline(always)]
fn decode_month(dr: &dr::R) -> u8 {
    let mt: u8 = if dr.mt().bit() { 1 } else { 0 };
    bcd2_decode(mt, dr.mu().bits()) as u8
}

#[inline(always)]
fn decode_year(dr: &dr::R) -> u16 {
    let year = bcd2_decode(dr.yt().bits(), dr.yu().bits()) + 1970; // 1970-01-01 is the epoch begin.
    year as u16
}
