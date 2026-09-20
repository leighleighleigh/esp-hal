pub use riscv::interrupt::{Exception, Interrupt};
pub use riscv_rt::{core_interrupt, exception, external_interrupt};

#[cfg(any(esp32s2, esp32s3))]
mod critical_section;
#[cfg(any(esp32s2, esp32s3))]
mod ulp_core;
#[cfg(any(esp32s2, esp32s3))]
pub use ulp_core::ExternalInterrupt;
