//! Power Control (PWR) peripheral
//!
//! This module provides an interface to configure the STM32L1 power management features,
//! including low-power modes (STOP, STANDBY) and voltage regulator settings.

use crate::stm32::PWR;
use cortex_m::peripheral::SCB;

/// Extension trait for the PWR peripheral
pub trait PwrExt {
    /// Constrains the PWR peripheral to play nicely with the other abstractions
    fn constrain(self) -> Pwr;
}

impl PwrExt for PWR {
    fn constrain(self) -> Pwr {
        Pwr { rb: self }
    }
}

/// Constrained PWR peripheral
pub struct Pwr {
    rb: PWR,
}

impl Pwr {
    /// Enable access to the backup domain (RTC, backup registers)
    pub fn enable_backup_access(&mut self) {
        self.rb.cr.modify(|_, w| w.dbp().set_bit());
    }

    /// Disable access to the backup domain
    pub fn disable_backup_access(&mut self) {
        self.rb.cr.modify(|_, w| w.dbp().clear_bit());
    }

    /// Clear the wakeup flag
    pub fn clear_wakeup_flag(&mut self) {
        self.rb.cr.modify(|_, w| w.cwuf().set_bit());
    }

    /// Clear the standby flag
    pub fn clear_standby_flag(&mut self) {
        self.rb.cr.modify(|_, w| w.csbf().set_bit());
    }

    /// Check if the wakeup flag is set
    pub fn is_wakeup_flag_set(&self) -> bool {
        self.rb.csr.read().wuf().bit_is_set()
    }

    /// Check if the standby flag is set
    pub fn is_standby_flag_set(&self) -> bool {
        self.rb.csr.read().sbf().bit_is_set()
    }

    /// Enter STOP mode with configurable options
    ///
    /// # Arguments
    /// * `config` - Stop mode configuration
    /// * `scb` - System Control Block for setting SLEEPDEEP
    ///
    /// # Example
    /// ```no_run
    /// let config = StopModeConfig {
    ///     ultra_low_power: true,
    ///     voltage_regulator: VoltageRegulator::LowPower,
    /// };
    /// pwr.stop_mode(config, &mut scb);
    /// cortex_m::asm::wfi();
    /// ```
    pub fn stop_mode(&mut self, config: StopModeConfig, scb: &mut SCB) {
        // Configure voltage regulator
        match config.voltage_regulator {
            VoltageRegulator::MainMode => {
                self.rb.cr.modify(|_, w| w.lpsdsr().clear_bit());
            }
            VoltageRegulator::LowPower => {
                self.rb.cr.modify(|_, w| w.lpsdsr().set_bit());
            }
        }

        // Configure ultra-low-power mode
        if config.ultra_low_power {
            self.rb.cr.modify(|_, w| w.ulp().set_bit());
        } else {
            self.rb.cr.modify(|_, w| w.ulp().clear_bit());
        }

        // Select STOP mode (not STANDBY)
        self.rb.cr.modify(|_, w| w.pdds().clear_bit());

        // Set SLEEPDEEP bit
        scb.set_sleepdeep();
    }

    /// Enter STANDBY mode
    ///
    /// # Arguments
    /// * `scb` - System Control Block for setting SLEEPDEEP
    ///
    /// # Safety
    /// After entering STANDBY mode, the device will reset upon wakeup.
    /// All SRAM and register contents will be lost except for backup domain.
    pub fn standby_mode(&mut self, scb: &mut SCB) {
        // Select STANDBY mode
        self.rb.cr.modify(|_, w| w.pdds().set_bit());

        // Set SLEEPDEEP bit
        scb.set_sleepdeep();
    }

    /// Enter SLEEP mode (regular sleep, not deep sleep)
    ///
    /// # Arguments
    /// * `scb` - System Control Block for clearing SLEEPDEEP
    ///
    /// In SLEEP mode, only the CPU is stopped. All peripherals continue to operate.
    pub fn sleep_mode(&mut self, scb: &mut SCB) {
        // Clear SLEEPDEEP bit
        scb.clear_sleepdeep();
    }

    /// Configure wakeup pin 1
    ///
    /// # Arguments
    /// * `enable` - Enable or disable wakeup pin 1
    pub fn configure_wakeup_pin1(&mut self, enable: bool) {
        self.rb.csr.modify(|_, w| w.ewup1().bit(enable));
    }

    /// Configure wakeup pin 2
    ///
    /// # Arguments
    /// * `enable` - Enable or disable wakeup pin 2
    pub fn configure_wakeup_pin2(&mut self, enable: bool) {
        self.rb.csr.modify(|_, w| w.ewup2().bit(enable));
    }

    /// Configure wakeup pin 3
    ///
    /// # Arguments
    /// * `enable` - Enable or disable wakeup pin 3
    pub fn configure_wakeup_pin3(&mut self, enable: bool) {
        self.rb.csr.modify(|_, w| w.ewup3().bit(enable));
    }

    /// Enable ultra-low-power mode (disables voltage reference and temperature sensor)
    pub fn enable_ultra_low_power(&mut self) {
        self.rb.cr.modify(|_, w| w.ulp().set_bit());
    }

    /// Disable ultra-low-power mode
    pub fn disable_ultra_low_power(&mut self) {
        self.rb.cr.modify(|_, w| w.ulp().clear_bit());
    }

    /// Configure PVD (Programmable Voltage Detector) level
    ///
    /// # Arguments
    /// * `level` - PVD threshold level
    pub fn configure_pvd(&mut self, level: PvdLevel) {
        self.rb
            .cr
            .modify(|_, w| unsafe { w.pls().bits(level as u8) });
    }

    /// Enable PVD (Programmable Voltage Detector)
    pub fn enable_pvd(&mut self) {
        self.rb.cr.modify(|_, w| w.pvde().set_bit());
    }

    /// Disable PVD (Programmable Voltage Detector)
    pub fn disable_pvd(&mut self) {
        self.rb.cr.modify(|_, w| w.pvde().clear_bit());
    }

    /// Get reference to the underlying PWR peripheral
    pub fn free(self) -> PWR {
        self.rb
    }
}

/// STOP mode configuration
#[derive(Debug, Clone, Copy)]
pub struct StopModeConfig {
    /// Enable ultra-low-power mode (disables voltage reference and temperature sensor)
    pub ultra_low_power: bool,
    /// Voltage regulator mode during STOP
    pub voltage_regulator: VoltageRegulator,
}

impl Default for StopModeConfig {
    fn default() -> Self {
        Self {
            ultra_low_power: false,
            voltage_regulator: VoltageRegulator::MainMode,
        }
    }
}

impl StopModeConfig {
    /// Create a new STOP mode configuration with ultra-low-power enabled
    pub fn ultra_low_power() -> Self {
        Self {
            ultra_low_power: true,
            voltage_regulator: VoltageRegulator::LowPower,
        }
    }

    /// Create a new STOP mode configuration with standard low-power
    pub fn low_power() -> Self {
        Self {
            ultra_low_power: false,
            voltage_regulator: VoltageRegulator::LowPower,
        }
    }

    /// Create a new STOP mode configuration with main voltage regulator
    pub fn main_mode() -> Self {
        Self {
            ultra_low_power: false,
            voltage_regulator: VoltageRegulator::MainMode,
        }
    }

    /// Enable ultra-low-power mode
    pub fn with_ultra_low_power(mut self, enable: bool) -> Self {
        self.ultra_low_power = enable;
        self
    }

    /// Set voltage regulator mode
    pub fn with_voltage_regulator(mut self, regulator: VoltageRegulator) -> Self {
        self.voltage_regulator = regulator;
        self
    }
}

/// Voltage regulator mode during STOP
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoltageRegulator {
    /// Main voltage regulator ON during STOP mode (faster wakeup, higher power consumption)
    MainMode,
    /// Low-power voltage regulator ON during STOP mode (slower wakeup, lower power consumption)
    LowPower,
}

/// PVD (Programmable Voltage Detector) threshold levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PvdLevel {
    /// 1.9V
    V1_9 = 0b000,
    /// 2.1V
    V2_1 = 0b001,
    /// 2.3V
    V2_3 = 0b010,
    /// 2.5V
    V2_5 = 0b011,
    /// 2.7V
    V2_7 = 0b100,
    /// 2.9V
    V2_9 = 0b101,
    /// 3.1V
    V3_1 = 0b110,
    /// External input on PVD_IN pin
    External = 0b111,
}
