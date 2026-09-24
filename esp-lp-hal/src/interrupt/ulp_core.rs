use riscv::{
    ExceptionNumber,
    InterruptNumber,
    interrupt::{Exception, Interrupt},
    result::*,
};
use riscv_rt::TrapFrame;

#[doc(hidden)]
#[unsafe(link_section = ".ulp_trap.rust")]
#[inline(always)]
pub fn ulp_setup_interrupts() {
    mie_impl(true);
}

#[doc(hidden)]
#[inline(always)]
#[unsafe(link_section = ".ulp_trap.rust")]
pub fn mie_impl(enable: bool) -> bool {
    // Does not affect the internal exception bits,
    //  which are always enabled (unmasked, value here is 0).
    // IRQ Type   Bit   Description
    // Internal     0   Internal timer interrupt
    // Internal     1   EBREAK/ECALL or Illegal Instruction
    // Internal     2   BUS Error (Unaligned Memory Access)
    // External    31   RTC peripheral interrupts
    let old_mask: u32;

    // let disable_exceptions: u32 = 0b111;
    let disable_bit: u32 = 1 << 31;

    // Create the new IRQ disable mask.
    // We will only change bit 31 in response to the enable boolean.
    // i.e. exceptions will always be enabled.
    let new_mask: u32 = if enable { 0b0 } else { disable_bit };

    unsafe {
        core::arch::asm!(
            "maskirq_insn {}, {}",
            out(reg) old_mask,
            in(reg) new_mask
        );
    }

    // Return previous enabled value,
    // where enable == 0 (unmasked)
    (old_mask & disable_bit) == 0
}

/// Converts an IRQ bitmask value into a riscv-rt-compatible
/// interrrupt::Trap type (Interrupt or Exception).
#[inline(always)]
#[unsafe(link_section = ".ulp_trap.rust")]
pub fn irq_to_mcause(cause: u32) -> Option<riscv::interrupt::Trap<usize, usize>> {
    // IRQ Type   Bit   Description                             Result
    // Internal     0   Internal timer interrupt                Interrupt::MachineTimer
    // Internal     1   EBREAK/ECALL or Illegal Instruction     Exception::IllegalInstruction
    // Internal     2   BUS Error (Unaligned Memory Access)     Exception::LoadMisaligned
    // External    31   RTC peripheral interrupts               Interrupt::MachineExternal

    // The mcause register does not exist on the riscv ULP cores,
    // so the Trap IRQ mask is read from custom register Q1.
    // let cause: u32;
    // unsafe {
    //     core::arch::asm!(
    //         "getq_insn {}, q1",
    //         out(reg) cause
    //     );
    // }

    if cause & (1 << 31) != 0 {
        return Some(riscv::interrupt::Trap::Interrupt(
            Interrupt::MachineExternal.number(),
        ));
    }

    if cause & (1 << 0) != 0 {
        return Some(riscv::interrupt::Trap::Interrupt(
            Interrupt::MachineTimer.number(),
        ));
    }

    if cause & (1 << 1) != 0 {
        return Some(riscv::interrupt::Trap::Exception(
            Exception::IllegalInstruction.number(),
        ));
    }

    if cause & (1 << 2) != 0 {
        return Some(riscv::interrupt::Trap::Exception(
            Exception::LoadMisaligned.number(),
        ));
    }

    None
}

#[doc(hidden)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn default_debug_start_trap(_trap_frame: *const TrapFrame, _irqs: u32) {
    // User code may override this function like so:
    // #[unsafe(export_name = "debug_start_trap")]
    // fn my_debug_start_trap(trap_frame : *const TrapFrame, irqs : u32) {
    // ...
    // }
}

#[doc(hidden)]
#[unsafe(link_section = ".ulp_trap.rust")]
#[unsafe(export_name = "_ulp_start_trap_rust")]
pub unsafe extern "C" fn ulp_start_trap_rust(trap_frame: *const TrapFrame, irqs: u32) {
    unsafe extern "C" {
        // These functions are created by the riscv-macros crate:
        // https://github.com/rust-embedded/riscv/blob/b3a70b7945f229e828d87dbd7e003cec291db23a/riscv-macros/src/riscv.rs#L242
        fn _dispatch_core_interrupt(code: usize);
        fn _dispatch_exception(trap_frame: *const TrapFrame, code: usize);
        // This debug function hook is called on the start of the trap,
        // user code may re-define it for debugging purposes.
        fn debug_start_trap(trap_frame: *const TrapFrame, irqs: u32);
    }

    unsafe {
        // Call debug function with the trap_frame and IRQ status.
        debug_start_trap(&*trap_frame, irqs);

        // Convert the irq bitmask to a riscv Trap type, and dispatch it to the handlers
        // registered in the `__EXCEPTIONS` or `__CORE_INTERRUPTS` arrays.
        // The `_dispatch_...` functions, and the ISR arrays, are provided/managed by `riscv-rt`.
        if let Some(mcause) = irq_to_mcause(irqs) {
            // Handle the trap
            match mcause {
                riscv::interrupt::Trap::Interrupt(code) => _dispatch_core_interrupt(code),
                riscv::interrupt::Trap::Exception(code) => _dispatch_exception(&*trap_frame, code),
            }
        }
    }
}

/// Just a dummy type to test the `ExternalInterrupt` trait.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ExternalInterrupt {
    /// Interrupt from an SAR peripheral
    SAR,
    /// Interrupt from the RTC_IO peripheral
    GPIO,
}

unsafe impl riscv::InterruptNumber for ExternalInterrupt {
    const MAX_INTERRUPT_NUMBER: usize = 1;

    #[inline]
    fn number(self) -> usize {
        self as usize
    }

    #[inline]
    fn from_number(value: usize) -> Result<Self> {
        match value {
            0 => Ok(Self::SAR),
            1 => Ok(Self::GPIO),
            _ => Err(Error::InvalidVariant(value)),
        }
    }
}
unsafe impl riscv::ExternalInterruptNumber for ExternalInterrupt {}
