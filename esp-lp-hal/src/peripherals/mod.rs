#[cfg(esp32c6)]
#[doc(hidden)]
pub mod lp_core_c6;
#[cfg(esp32c6)]
pub use lp_core_c6::*;

#[cfg(esp32s2)]
#[doc(hidden)]
pub mod ulp_core_s2;
#[cfg(esp32s2)]
pub use ulp_core_s2::*;

#[cfg(esp32s3)]
#[doc(hidden)]
pub mod ulp_core_s3;
#[cfg(esp32s3)]
pub use ulp_core_s3::*;