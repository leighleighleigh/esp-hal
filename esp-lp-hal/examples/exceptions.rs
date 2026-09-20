//! ULP interrupt-based counter example.
//! Increments a 32 bit counter value at a known point in memory, whenever the ULP program is run.
//! If GPIO0 is pressed, resets the counter.

//% CHIPS: esp32s3 esp32s2

#![no_std]
#![no_main]

extern crate panic_halt;

use esp_lp_hal::{
    interrupt::{Exception, ExternalInterrupt, exception, external_interrupt},
    pac::Peripherals,
    prelude::*,
};

const ADDRESS: usize = 0x1000;

#[entry]
fn main() {
    let _peripherals = Peripherals::take().unwrap();

    loop {
        unsafe {
            core::arch::asm!("nop");
        }
    }
}

// /// Handler with the simplest signature.
// #[external_interrupt(ExternalInterrupt::SAR)]
// fn external_gpio() {
//     // Increment the counter every time RISCV_START_INT is triggered
//     unsafe {
//         let counter = ADDRESS as *mut u32;
//         counter.write_volatile(counter.read_volatile() + 1);
//     }
//     // do something here
//     // loop {}
// }

/// Handler with the most complete signature.
#[exception(Exception::IllegalInstruction)]
unsafe fn illegal_instruction(_trap: &riscv_rt::TrapFrame) -> ! {
    // Increment the counter every time RISCV_START_INT is triggered
    unsafe {
        let counter = ADDRESS as *mut u32;
        counter.write_volatile(counter.read_volatile() + 1);
    }
    // do something here
    loop {}
}
