#[cfg(any(esp32s2, esp32s3))]
mod critical_section;
#[cfg(any(esp32s2, esp32s3))]
mod ulp_core;
#[cfg(any(esp32s2, esp32s3))]
pub use ulp_core::*;
