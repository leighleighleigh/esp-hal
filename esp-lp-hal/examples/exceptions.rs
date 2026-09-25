//! ULP exception trap example.
//! When an IllegalInstruction occurs, will write '0xdeaddead' to *ADDRESS.

//% CHIPS: esp32s3 esp32s2

#![no_std]
#![no_main]

extern crate panic_halt;

use esp_lp_hal::{delay::Delay, interrupt::Exception, pac::Peripherals, prelude::*};
use riscv_rt::exception;

const ADDRESS: usize = 0x1000;

#[entry]
fn main() {
    let _peripherals = Peripherals::take().unwrap();

    // Delay for a second
    let dly = Delay {};
    dly.delay_millis(1000);

    // Crash
    unsafe {
        core::arch::asm!("csrrs a1, mcause, zero");
    }

    loop {}
}

#[exception(Exception::IllegalInstruction)]
unsafe fn illegal_instruction(_trap: &riscv_rt::TrapFrame) -> ! {
    unsafe {
        let reg = ADDRESS as *mut u32;
        reg.write_volatile(0xdeaddead);
    }
    loop {}
}
