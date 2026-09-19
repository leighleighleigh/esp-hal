pub use riscv::interrupt::{Exception, Interrupt};
use riscv::result::*;
pub use riscv_rt::{core_interrupt, exception, external_interrupt};

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

#[unsafe(export_name = "DefaultHandler")]
unsafe fn custom_interrupt_handler() {
    loop {}
}

// /// Handler with the simplest signature.
// #[core_interrupt(Interrupt::SupervisorSoft)]
// fn supervisor_soft() {
//     // do something here
//     loop {}
// }

// /// Handler with the most complete signature.
// #[core_interrupt(Interrupt::SupervisorTimer)]
// unsafe fn supervisor_timer() -> ! {
//     // do something here
//     loop {}
// }

// // EXAMPLES OF USING THE exception MACRO FOR EXCEPTION HANDLERS.

// /// Handler with the simplest signature.
// #[exception(Exception::InstructionMisaligned)]
// fn instruction_misaligned() {
//     // do something here
//     loop {}
// }

// /// Handler with the most complete signature.
// #[exception(Exception::IllegalInstruction)]
// unsafe fn illegal_instruction(_trap: &riscv_rt::TrapFrame) -> ! {
//     // do something here
//     loop {}
// }

// // The reference to TrapFrame can be mutable if the handler needs to modify it.
// #[exception(Exception::Breakpoint)]
// unsafe fn breakpoint(_trap: &mut riscv_rt::TrapFrame) -> ! {
//     // do something here
//     loop {}
// }
