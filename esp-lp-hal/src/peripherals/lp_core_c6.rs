use crate::pac::{LP_I2C0,LP_PERI,LP_ANA,LP_AON,LP_APM,LP_CLKRST,LP_I2C_ANA_MST,LP_IO,LP_TEE,LP_TIMER,LP_UART,LP_WDT,PMU};

#[unsafe(no_mangle)]
static mut DEVICE_PERIPHERALS: bool = false;
#[doc = r" All the peripherals."]
#[allow(non_snake_case)]
pub struct Peripherals {
    #[doc = "LP_I2C0"]
    pub LP_I2C0: LP_I2C0,
    #[doc = "LP_PERI"]
    pub LP_PERI: LP_PERI,
    #[doc = "LP_ANA"]
    pub LP_ANA: LP_ANA,
    #[doc = "LP_AON"]
    pub LP_AON: LP_AON,
    #[doc = "LP_APM"]
    pub LP_APM: LP_APM,
    #[doc = "LP_CLKRST"]
    pub LP_CLKRST: LP_CLKRST,
    #[doc = "LP_I2C_ANA_MST"]
    pub LP_I2C_ANA_MST: LP_I2C_ANA_MST,
    #[doc = "LP_IO"]
    pub LP_IO: LP_IO,
    #[doc = "LP_TEE"]
    pub LP_TEE: LP_TEE,
    #[doc = "LP_TIMER"]
    pub LP_TIMER: LP_TIMER,
    #[doc = "LP_UART"]
    pub LP_UART: LP_UART,
    #[doc = "LP_WDT"]
    pub LP_WDT: LP_WDT,
    #[doc = "PMU"]
    pub PMU: PMU,
}
impl Peripherals {
    #[doc = r" Returns all the peripherals *once*."]
    #[inline]
    pub fn take() -> Option<Self> {
        critical_section::with(|_| {
            if unsafe { DEVICE_PERIPHERALS } {
                return None;
            }
            Some(unsafe { Peripherals::steal() })
        })
    }
    #[doc = r" Unchecked version of `Peripherals::take`."]
    #[doc = r""]
    #[doc = r" # Safety"]
    #[doc = r""]
    #[doc = r" Each of the returned peripherals must be used at most once."]
    #[allow(unsafe_op_in_unsafe_fn)]
    #[inline]
    pub unsafe fn steal() -> Self {
        DEVICE_PERIPHERALS = true;
        Peripherals {
            LP_I2C0: LP_I2C0::steal(),
            LP_PERI: LP_PERI::steal(),
            LP_ANA: LP_ANA::steal(),
            LP_AON: LP_AON::steal(),
            LP_APM: LP_APM::steal(),
            LP_CLKRST: LP_CLKRST::steal(),
            LP_I2C_ANA_MST: LP_I2C_ANA_MST::steal(),
            LP_IO: LP_IO::steal(),
            LP_TEE: LP_TEE::steal(),
            LP_TIMER: LP_TIMER::steal(),
            LP_UART: LP_UART::steal(),
            LP_WDT: LP_WDT::steal(),
            PMU: PMU::steal(),
        }
    }
}