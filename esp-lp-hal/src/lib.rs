#![cfg_attr(
    all(docsrs, not(not_really_docsrs)),
    doc = "<div style='padding:30px;background:#810;color:#fff;text-align:center;'><p>You might want to <a href='https://docs.espressif.com/projects/rust/'>browse the <code>esp-lp-hal</code> documentation on the esp-rs website</a> instead.</p><p>The documentation here on <a href='https://docs.rs'>docs.rs</a> is built for a single chip only (ESP32-C6, in particular), while on the esp-rs website you can select your exact chip from the list of supported devices. Available peripherals and their APIs change depending on the chip.</p></div>\n\n<br/>\n\n"
)]
//! Bare-metal (`no_std`) HAL for the low power and ultra-low power cores found
//! in some Espressif devices. Where applicable, drivers implement the
//! [embedded-hal] traits.
//!
//! ## Choosing a device
//!
//! Depending on your target device, you need to enable the chip feature
//! for that device.
//!
//! ## Feature Flags
#![doc = document_features::document_features!(feature_label = r#"<span class="stab portability"><code>{feature}</code></span>"#)]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/46717278")]
#![allow(asm_sub_register)]
#![deny(missing_docs)]
#![no_std]

#[allow(unused_imports, reason = "Only used for some MCUs currently")]
#[macro_use]
extern crate esp_metadata_generated;

use core::arch::global_asm;

pub mod delay;
pub mod gpio;
#[cfg(lp_i2c_master)]
pub mod i2c;
#[cfg(lp_uart)]
pub mod uart;

#[cfg(feature = "esp32c6")]
pub use esp32c6_lp as pac;
#[cfg(feature = "esp32s2")]
pub use esp32s2_ulp as pac;
#[cfg(feature = "esp32s3")]
pub use esp32s3_ulp as pac;

/// The prelude
pub mod prelude {
    pub use procmacros::entry;
}

cfg_if::cfg_if! {
    if #[cfg(feature = "esp32c6")] {
        // LP_FAST_CLK is not very accurate, for now use a rough estimate
        const LP_FAST_CLK_HZ: u32 = 16_000_000;
        const XTAL_D2_CLK_HZ: u32 = 20_000_000;
    } else if #[cfg(feature = "esp32s2")] {
        const LP_FAST_CLK_HZ: u32 = 8_000_000;
    } else if #[cfg(feature = "esp32s3")] {
        const LP_FAST_CLK_HZ: u32 = 17_500_000;
    }
}

pub(crate) static mut CPU_CLOCK: u32 = LP_FAST_CLK_HZ;

/// Wake up the HP core
#[cfg(feature = "esp32c6")]
pub fn wake_hp_core() {
    unsafe { &*esp32c6_lp::PMU::PTR }
        .hp_lp_cpu_comm()
        .write(|w| w.lp_trigger_hp().set_bit());
}

/// Wake up the HP core
#[cfg(any(feature = "esp32s2", feature = "esp32s3"))]
#[unsafe(link_section = ".init.rust")]
#[unsafe(no_mangle)]
pub fn wake_hp_core() {
    unsafe { &*pac::RTC_CNTL::PTR }
        .rtc_state0()
        .write(|w| w.rtc_sw_cpu_int().set_bit());
}

#[cfg(feature = "esp32c6")]
global_asm!(
    r#"
    .section    .init.vector, "ax"
    /* This is the vector table. It is currently empty, but will be populated
     * with exception and interrupt handlers when this is supported
     */

    .align  0x4, 0xff
    .global _vector_table
    .type _vector_table, @function
_vector_table:
    .option push
    .option norvc

    .rept 32
    nop
    .endr

    .option pop
    .size _vector_table, .-_vector_table

    .section .init, "ax"
    .global reset_vector

/* The reset vector, jumps to startup code */
reset_vector:
    j __start

__start:
    /* setup the stack pointer */
    la sp, __stack_top
    call rust_main
loop:
    j loop
"#
);

#[cfg(any(feature = "esp32s2", feature = "esp32s3"))]
global_asm!(
    r#"
  .equ SAVE_REGS, 17
  .equ CONTEXT_SIZE, (SAVE_REGS * 4)

  /* Much of this assembly was sourced from the following ESP-IDF files...
  *  ...irq handler macros:
  *    https://github.com/espressif/esp-idf/blob/master/components/ulp/ulp_riscv/ulp_core/ulp_riscv_vectors.S 
  *
  *  ...critical section assembly
  *    https://github.com/espressif/esp-idf/blob/master/components/ulp/ulp_riscv/ulp_core/include/ulp_riscv_utils.h
  *
  *  ...riscv halt code
  *    https://github.com/espressif/esp-idf/blob/master/components/ulp/ulp_riscv/ulp_core/ulp_riscv_utils.c
  */

  /* Macro which first allocates space on the stack to save general
   * purpose registers, and then save them. GP register is excluded.
   * The default size allocated on the stack is CONTEXT_SIZE, but it
   * can be overridden.
   *
   * Note: We don't save the callee-saved s0-s11 registers to save space
   */
  .macro save_general_regs cxt_size=CONTEXT_SIZE
      addi sp, sp, -\cxt_size
      sw   ra, 0(sp)
      sw   tp, 4(sp)
      sw   t0, 8(sp)
      sw   t1, 12(sp)
      sw   t2, 16(sp)
      sw   a0, 20(sp)
      sw   a1, 24(sp)
      sw   a2, 28(sp)
      sw   a3, 32(sp)
      sw   a4, 36(sp)
      sw   a5, 40(sp)
      sw   a6, 44(sp)
      sw   a7, 48(sp)
      sw   t3, 52(sp)
      sw   t4, 56(sp)
      sw   t5, 60(sp)
      sw   t6, 64(sp)
  .endm

  /* Restore the general purpose registers (excluding gp) from the context on
   * the stack. The context is then deallocated. The default size is CONTEXT_SIZE
   * but it can be overridden. */
  .macro restore_general_regs cxt_size=CONTEXT_SIZE
      lw   ra, 0(sp)
      lw   tp, 4(sp)
      lw   t0, 8(sp)
      lw   t1, 12(sp)
      lw   t2, 16(sp)
      lw   a0, 20(sp)
      lw   a1, 24(sp)
      lw   a2, 28(sp)
      lw   a3, 32(sp)
      lw   a4, 36(sp)
      lw   a5, 40(sp)
      lw   a6, 44(sp)
      lw   a7, 48(sp)
      lw   t3, 52(sp)
      lw   t4, 56(sp)
      lw   t5, 60(sp)
      lw   t6, 64(sp)
      addi sp,sp, \cxt_size
  .endm

  .section .text.vectors
  .global irq_vector
  .global reset_vector
  .global ulp_irq_handler
  
  /* The reset vector, jumps to startup code */
  reset_vector:
    j __start

  /* Interrupt handler */
  .balign 0x10 
  irq_vector:
    /* Save the general gurpose register context before handling the interrupt */
    save_general_regs
    /* Fetch the interrupt status from the custom q1 register into a0 */
    /* getq_insn(a0, q1) */
    /* .word (((0b0000000) << 25) | ((0) << 20) | ((1) << 15) | ((0b100) << 12) | ((10) << 7) | ((0b0001011) << 0)) */
    .word 0x0000C50B

    /* Call the global C interrupt handler. The interrupt status is passed as the argument in a0.
     * We do not re-enable interrupts before calling the C handler as ULP RISC-V does not
     * support nested interrupts.
     */
    jal ulp_irq_handler

    /* Restore the register context after returning from the C interrupt handler */
    restore_general_regs

    /* Exit interrupt handler by executing the custom retirq instruction which will restore pc and re-enable interrupts */
    /* retirq_insn() */
    /* .word (((0b0000010) << 25) | ((0) << 20) | ((0) << 15) | ((0b000) << 12) | ((0) << 7) | ((0b0001011) << 0)); */
    .word 0x0400000B

	.section .text

  __start:
    /* setup the stack pointer */
    la sp, __stack_top
    call ulp_riscv_rescue_from_monitor
    /* Wait for any interrupt */
    /* waitirq x0 */
    /* .word 0x0800400B */
    /* Enable interrupts globally */
    /* maskirq_insn(zero, zero) */
    /* .word 0x0600600b */
    call rust_main
    call ulp_riscv_halt

  loop:
    j loop
  "#
);

#[unsafe(link_section = ".init.rust")]
#[unsafe(export_name = "rust_main")]
unsafe extern "C" fn lp_core_startup() -> ! {
    unsafe {
        unsafe extern "Rust" {
            fn main() -> !;
        }

        #[cfg(feature = "esp32c6")]
        if (*pac::LP_CLKRST::PTR)
            .lp_clk_conf()
            .read()
            .fast_clk_sel()
            .bit_is_set()
        {
            CPU_CLOCK = XTAL_D2_CLK_HZ;
        }

        #[cfg(feature = "stack-guard")]
        setup_stack_guard(0xdeadbabe);

        main();
    }
}

/// Enter a critical section (disable interrupts)
#[cfg(any(feature = "esp32s2", feature = "esp32s3"))]
#[unsafe(link_section = ".init.rust")]
#[inline(always)]
pub fn ulp_disable_interrupts() {
    // Enter a critical section by disabling all interrupts
    // This inline assembly construct uses the t0 register and is equivalent to:
    // > li t0, 0x80000007
    // > maskirq_insn(zero, t0) // Mask all interrupt bits
    //
    // The mask 0x80000007 represents:
    //   Bit 31 - RTC peripheral interrupt
    //   Bit 2  - Bus error
    //   Bit 1  - Ebreak / Ecall / Illegal Instruction
    //   Bit 0  - Internal Timer
    //
    unsafe {
        core::arch::asm!("li t0, 0x80000007", ".word 0x0602e00b");
    }
}

/// Exit a critical section (re-enable interrupts)
#[cfg(any(feature = "esp32s2", feature = "esp32s3"))]
#[unsafe(link_section = ".init.rust")]
#[inline(always)]
pub fn ulp_enable_interrupts() {
    // Exit a critical section by enabling all interrupts
    // This inline assembly construct is equivalent to:
    // > maskirq_insn(zero, zero)  // Unmask all interrupt bits
    unsafe {
        core::arch::asm!(".word 0x0600600b");
    }
}

/// Wait for any (even unmasked) interrupt
#[cfg(any(feature = "esp32s2", feature = "esp32s3"))]
#[unsafe(link_section = ".init.rust")]
#[inline(always)]
pub fn ulp_waitirq() {
    // Wait for interrupt
    // waitirq x0
    unsafe {
        core::arch::asm!(".word 0x0800400B");
    }
}

#[cfg(feature = "stack-guard")]
#[cfg(any(feature = "esp32s2", feature = "esp32s3"))]
#[unsafe(link_section = ".init.rust")]
/// Writes an expected value to __stack_chk_guard
pub fn setup_stack_guard(value: u32) {
    unsafe extern "C" {
        static mut __stack_chk_guard: u32;
    }
    unsafe {
        let stack_chk_guard = core::ptr::addr_of_mut!(__stack_chk_guard);
        stack_chk_guard.write_unaligned(value);
    }
}

/// Inner implimentation of the IRQ handler
#[cfg(any(feature = "esp32s2", feature = "esp32s3"))]
#[unsafe(link_section = ".init.rust")]
#[unsafe(no_mangle)]
#[inline(always)]
pub unsafe extern "C" fn ulp_irq_handler_impl(q1: u32) {
    // This is where we can handle stuff!
    // Does nothing at the moment, but is declared weak - so can be overriden by user.
    // Reference implimentation:
    // https://github.com/espressif/esp-idf/blob/master/components/ulp/ulp_riscv/ulp_core/ulp_riscv_interrupt.c
}

#[cfg(any(feature = "esp32s2", feature = "esp32s3"))]
#[unsafe(link_section = ".init.rust")]
#[unsafe(no_mangle)]
unsafe extern "C" fn ulp_riscv_rescue_from_monitor() {
    // Rescue RISC-V core from monitor state.
    unsafe { &*pac::RTC_CNTL::PTR }
        .cocpu_ctrl()
        .modify(|_, w| w.cocpu_done().clear_bit().cocpu_shut_reset_en().clear_bit());

    //// Enable the start interrupt - which happens when the chip starts.
    //unsafe { &*pac::SENS::PTR }
    //    .sar_cocpu_int_ena()
    //    .write(|w| w.sar_cocpu_start_int_ena().set_bit());
}

#[cfg(feature = "stack-guard")]
#[cfg(any(feature = "esp32s2", feature = "esp32s3"))]
#[unsafe(export_name = "__stack_chk_fail")]
unsafe extern "C" fn stack_chk_fail() {
    //panic!("Stack corruption detected");
}

/// Stops the ULP core, called from itself.
#[cfg(any(feature = "esp32s2", feature = "esp32s3"))]
#[unsafe(link_section = ".init.rust")]
#[unsafe(no_mangle)]
unsafe extern "C" fn ulp_riscv_halt() -> ! {
    unsafe { &*pac::RTC_CNTL::PTR }
        .cocpu_ctrl()
        .write(|w| unsafe {
            w.cocpu_shut_2_clk_dis()
                .bits(0x3f)
                .cocpu_done()
                .set_bit()
                .cocpu_shut_reset_en()
                .set_bit()
        });
    loop {
        //   riscv::asm::wfi();
        ulp_waitirq();
    }
}
