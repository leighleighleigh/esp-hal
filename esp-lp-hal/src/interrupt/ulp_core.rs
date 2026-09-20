use riscv::result::*;
use riscv_rt::{TrapFrame, setup_interrupts};

#[doc(hidden)]
// Must be in .trap section, NOT .trap.rust, because we will be discarding .trap.rust !
#[unsafe(link_section = ".trap")]
#[inline(always)]
pub fn mie_impl(enable: bool) -> bool {
    // Does not affect the internal exception bits,
    //  which are always enabled (unmasked, value here is 0).
    // IRQ Type   Bit   Description
    // Internal     0   Internal timer interrupt
    // Internal     1   EBREAK/ECALL or Illegal Instruction
    // Internal     2   BUS Error (Unaligned Memory Access)
    // External    31   RTC peripheral interrupts
    let old_mask: u32;

    let disable_exceptions: u32 = 0b111;
    let disable_bit: u32 = 1 << 31;
    let new_mask: u32 = if enable {
        0b0
    } else {
        disable_bit | disable_exceptions
    };

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

/// Returns the cause of a machine interrupt,
/// which contains the exception code, and if a peripheral interrupt was flagged.
// Must be in .trap section, NOT .trap.rust, because we will be discarding .trap.rust !
#[unsafe(link_section = ".trap")]
#[inline(always)]
pub fn mcause_impl() -> riscv::interrupt::Trap<usize, usize> {
    // Exception ID, Description
    // 2, Illegal instructions
    // 3, Breakpoints (EBREAK)
    // 6, Misaligned atomic instructions
    //
    // Bit 31 is used to indicate an interrupt.

    // mcause register does not exist on ULP cores,
    // so the Trap must be formed using q-registers
    let cause: u32;
    unsafe {
        core::arch::asm!(
            "getq_insn {}, q1",
            out(reg) cause
        );
    }

    let interrupt: bool = (cause & (0b1 << 31)) != 0;

    if interrupt {
        let code = (cause & 0b111) as usize;
        riscv::interrupt::Trap::Interrupt(code)
    } else {
        let code = (cause & 0b1111) as usize;
        riscv::interrupt::Trap::Exception(code)
    }
}

#[setup_interrupts]
unsafe fn ulp_setup_interrupts() {
    // TODO: Enable interrupts
    mie_impl(true);
}

#[doc(hidden)]
// Must be in .trap section, NOT .trap.rust, because we will be discarding .trap.rust !
#[unsafe(link_section = ".trap")]
#[unsafe(export_name = "_ulp_start_trap_rust")]
pub unsafe extern "C" fn ulp_start_trap_rust(trap_frame: *const TrapFrame) {
    unsafe extern "C" {
        fn _dispatch_core_interrupt(code: usize);
        fn _dispatch_exception(trap_frame: &TrapFrame, code: usize);
    }

    unsafe {
        match mcause_impl() {
            riscv::interrupt::Trap::Interrupt(code) => _dispatch_core_interrupt(code),
            riscv::interrupt::Trap::Exception(code) => _dispatch_exception(&*trap_frame, code),
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
