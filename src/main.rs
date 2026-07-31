//! Minimal blink for the Raspberry Pi Pico 2 (RP2350).
//! The starting point for the rp2350-security-lab project.

#![no_std]
#![no_main]

// Halt on panic. Mentioning the crate ensures it gets linked.
use panic_halt as _;

// Alias the HAL crate
use rp235x_hal as hal;

// Traits we need for delay and GPIO
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;

/// Tell the Boot ROM about our application.
///
/// This block declares the image as a "secure ARM executable" in the
/// RP2350's boot metadata. It's the same `image type: ARM Secure` field
/// that `picotool info` reports. The boot ROM reads this to know how to
/// launch the firmware.
#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

/// The Pico 2's external crystal is 12 MHz.
const XTAL_FREQ_HZ: u32 = 12_000_000u32;

/// Program metadata for `picotool info`.
#[link_section = ".bi_entries"]
#[used]
pub static PICOTOOL_ENTRIES: [hal::binary_info::EntryAddr; 3] = [
    hal::binary_info::rp_program_name!(c"rp2350-security-lab"),
    hal::binary_info::rp_program_description!(c"Embedded Rust security learning on RP2350"),
    hal::binary_info::rp_program_build_attribute!(),
];

#[hal::entry]
fn main() -> ! {
    // Grab the peripherals
    let mut pac = hal::pac::Peripherals::take().unwrap();

    // Set up the watchdog + clocks
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

    // A delay object based on the system timer
    let mut timer = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);

    // Set up the GPIO pins
    let sio = hal::Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // The onboard LED is on GPIO25
    let mut led_pin = pins.gpio25.into_push_pull_output();

    // Blink forever
    loop {
        led_pin.set_high().unwrap();
        timer.delay_ms(500);
        led_pin.set_low().unwrap();
        timer.delay_ms(500);
    }
}
