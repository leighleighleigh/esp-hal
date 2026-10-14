use critical_section::RawRestoreState;

use super::ulp_core;

struct UlpCriticalSection;
critical_section::set_impl!(UlpCriticalSection);

/// Machine-mode critical section implementation for
/// LP and ULP cores.
unsafe impl critical_section::Impl for UlpCriticalSection {
    unsafe fn acquire() -> RawRestoreState {
        ulp_core::mie_impl(false)
    }

    unsafe fn release(_previous_state: RawRestoreState) {
        ulp_core::mie_impl(_previous_state);
    }
}
