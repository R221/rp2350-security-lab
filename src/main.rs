//! Tamper-response firmware for the Raspberry Pi Pico 2 (RP2350).
//!
//! Holds a "secret" (stand-in for a key) and ZEROIZES it the instant a
//! tamper switch on a GPIO pin trips. Demonstrates the anti-tamper loop:
//! detect -> respond -> zeroize.
//!
//! Wiring:
//!   Tamper switch between GPIO15 and GND.
//!   GPIO15 is input-with-pull-up: LOW = switch closed (lid on, safe),
//!   HIGH = switch open (lid removed, TAMPER).
//!   Onboard LED (GPIO25): ON = armed/secret intact, OFF = wiped.

#![no_std]
#![no_main]

use panic_halt as _;
use rp235x_hal as hal;

use core::ptr::write_volatile;
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};

/// Boot metadata: declares this a secure ARM image to the RP2350 boot ROM.
#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

/// Program metadata for `picotool info`.
#[link_section = ".bi_entries"]
#[used]
pub static PICOTOOL_ENTRIES: [hal::binary_info::EntryAddr; 3] = [
    hal::binary_info::rp_program_name!(c"rp2350-security-lab"),
    hal::binary_info::rp_program_description!(c"Tamper-response firmware: zeroize on GPIO trip"),
    hal::binary_info::rp_program_build_attribute!(),
];

const XTAL_FREQ_HZ: u32 = 12_000_000u32;

/// Size of our stand-in secret (e.g. a 256-bit key = 32 bytes).
const SECRET_LEN: usize = 32;

/// Overwrite the secret buffer with zeros using VOLATILE writes.
///
/// Why volatile: a plain `for b in secret { *b = 0; }` can be optimized
/// away by the compiler if it decides the buffer is never read again ("dead
/// store elimination"). That would mean the wipe silently does not happen —
/// a real cryptographic-hygiene bug. `write_volatile` forces the write to
/// actually occur. (Production code would use the `zeroize` crate.)
fn zeroize_secret(secret: &mut [u8; SECRET_LEN]) {
    for byte in secret.iter_mut() {
        unsafe {
            write_volatile(byte as *mut u8, 0u8);
        }
    }
}

#[hal::entry]
fn main() -> ! {
    let mut pac = hal::pac::Peripherals::take().unwrap();

    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);
    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .unwrap();

    let mut timer = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);

    let sio = hal::Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Onboard LED (GPIO25): status indicator.
    let mut led_pin = pins.gpio25.into_push_pull_output();

    // Tamper switch input on GPIO15, pulled up internally.
    // LOW  = switch closed to GND (lid on)  -> safe
    // HIGH = switch open (lid removed)       -> tamper
    let mut tamper_pin = pins.gpio15.into_pull_up_input();

    // --- Load the secret. In a real device this would be provisioned key
    // material; here it's a recognizable pattern so you can confirm the
    // wipe by dumping RAM before/after. ---
    let mut secret: [u8; SECRET_LEN] = [0xAB; SECRET_LEN];

    // Armed: secret intact.
    led_pin.set_high().unwrap();

    // Latch: once tampered, stay tampered. A real tamper response is one-way.
    let mut tampered = false;

    loop {
        if !tampered {
            // is_high() -> switch open -> lid removed -> TAMPER
            if tamper_pin.is_high().unwrap() {
                // RESPOND: destroy the secret.
                zeroize_secret(&mut secret);

                // Signal the wipe.
                led_pin.set_low().unwrap();
                tampered = true;

                // (Optional) prevent the compiler from deciding `secret` is
                // now unused and eliding earlier stores: read it via a
                // volatile black-box. Reading one byte volatile is enough to
                // keep the buffer "observed".
                let _observed = unsafe { core::ptr::read_volatile(&secret[0] as *const u8) };
            }
        }

        // Small poll interval. (An interrupt on the pin would be lower-power
        // and faster; polling keeps this first version simple.)
        timer.delay_ms(20);
    }
}