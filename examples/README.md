# Examples

This directory contains example applications demonstrating the features of the `stm32l1xx-hal` crate.

## Building Examples

To build an example:

```bash
cargo build --example <example_name> --features stm32l152 --target thumbv7m-none-eabi
```

Replace `stm32l152` with your target MCU feature (`stm32l100`, `stm32l151`, `stm32l152`, or `stm32l162`).

## Available Examples

### GPIO Examples
- **`blinky.rs`** - Simple LED blinking using busy-wait loops
- **`blinky_delay.rs`** - LED blinking using delay abstraction
- **`blinky_timer.rs`** - LED blinking using hardware timer
- **`button.rs`** - Reading button input
- **`button_irq.rs`** - Button input with interrupt handling
- **`gpio_interrupt.rs`** - GPIO interrupt configuration and handling

### Peripheral Examples
- **`adc.rs`** - Analog to Digital Converter (ADC) reading
- **`adc_pwm.rs`** - ADC combined with PWM output
- **`dac.rs`** - Digital to Analog Converter (DAC) output
- **`dma.rs`** - Direct Memory Access (DMA) usage
- **`i2c.rs`** - I2C communication
- **`pwm.rs`** - Pulse Width Modulation (PWM) output
- **`qei.rs`** - Quadrature Encoder Interface
- **`serial.rs`** - UART/Serial communication
- **`spi.rs`** - SPI communication
- **`timer.rs`** - Hardware timer with interrupts
- **`watchdog.rs`** - Watchdog timer configuration

### RTC Examples
- **`rtc_backup_simple.rs`** - RTC backup register usage and power-loss detection
  - Demonstrates backup register read/write
  - Shows how to detect VBAT power loss
  - Implements boot counter that persists across resets
  
- **`rtc_backup.rs`** - Complete RTC example with time keeping
  - RTC initialization with magic number check
  - Setting and reading date/time
  - Backup register for boot counting
  - RTC wakeup timer usage

### Low Power Examples
- **`low_power.rs`** - Deep sleep (STOP mode) with RTC wakeup
  - System clock configuration using HSE at 24MHz with PLL
  - RTC initialization with LSE (32.768 kHz external crystal)
  - STOP mode (deep sleep) with ultra-low-power mode enabled
  - Periodic wakeup using RTC wakeup timer
  - Automatic clock reconfiguration after wakeup
  - Wakeup counter stored in RTC backup registers
  - Demonstrates power-loss detection across sleep cycles

- **`low_power_advanced.rs`** - Advanced deep sleep with clock monitoring
  - All features from `low_power.rs`
  - Clock source verification before and after STOP mode
  - Demonstrates `get_sysclk_source()` for debugging
  - Shows detailed clock reconfiguration process
  - Useful for understanding STOP mode clock behavior

- **`power_modes.rs`** - Comparison of different power modes
  - Cycles through STOP (main regulator), STOP (low-power), STOP (ultra-low-power), and SLEEP
  - Shows power consumption vs wakeup time tradeoffs
  - Uses RTC backup registers to track mode counter
  - Educational example for choosing the right power mode

### Debug Examples
- **`hello.rs`** - Simple "Hello World" via semihosting
- **`itm.rs`** - ITM (Instrumentation Trace Macrocell) output
- **`rtic.rs`** - Real-Time Interrupt-driven Concurrency (RTIC) framework example

## RTC Backup Register Features

The RTC examples demonstrate new functionality added to the HAL:

### Power-Loss Detection
```rust
const MAGIC_NUMBER: u32 = 0x32F2;

if !rtc.is_initialized(0, MAGIC_NUMBER) {
    // First boot or VBAT was lost - initialize RTC
    rtc.set_datetime(&datetime)?;
    rtc.mark_initialized(0, MAGIC_NUMBER);
}
```

### Persistent Storage
RTC backup registers (0-31) retain data as long as VBAT is supplied:

```rust
// Read boot counter
let count = rtc.read_backup_register(1);

// Increment and save
rtc.write_backup_register(1, count + 1);
```

### Use Cases
- Boot counter
- Configuration flags that persist across resets
- Timestamps for power-loss events
- System state recovery after unexpected resets
- Calibration data that shouldn't be lost

## Hardware Requirements

Most examples require:
- STM32L1xx development board (e.g., STM32L152 Discovery)
- Debug probe (ST-Link V2 or compatible)
- For RTC examples: 32.768 kHz crystal (LSE) or coin cell battery on VBAT

## Running Examples

1. Connect your debug probe to the target board
2. Build the example
3. Flash using your preferred tool:
   ```bash
   # Using cargo-embed
   cargo embed --example rtc_backup_simple --features stm32l152
   
   # Using probe-run
   cargo run --example rtc_backup_simple --features stm32l152
   ```

## Troubleshooting

- If RTC examples don't work, ensure the LSE crystal is properly connected
- For backup register persistence, VBAT must be connected to a battery or VDD
- Some examples require semihosting support in your debug environment
