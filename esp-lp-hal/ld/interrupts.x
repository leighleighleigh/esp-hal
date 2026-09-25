/* Default abort entry point. If no abort symbol is provided, then abort maps to _default_abort. */
EXTERN(_default_abort);
PROVIDE(abort = _default_abort);

/* Default trap entry point. If not _start_trap symbol is provided, then _start_trap maps to
   _default_start_trap, which saves caller saved registers, calls _start_trap_rust, restores
   caller saved registers and then returns. Note that _start_trap must be 4-byte aligned */
EXTERN(_ulp_start_trap);
PROVIDE(_start_trap = _ulp_start_trap);

/* Trap start debug function */
EXTERN(default_debug_start_trap);
PROVIDE(debug_start_trap = default_debug_start_trap);

/* Default main routine. If no hal_main symbol is provided, then hal_main maps to main, which
   is usually defined by final users via the #[riscv_rt::entry] attribute. Using hal_main
   instead of main directly allow HALs to inject code before jumping to user main. */
EXTERN(_ulp_start_rust);
PROVIDE(hal_main = _ulp_start_rust);


/* Ensure the interrupt vector tables, and dispatch functions, are always included in the binary */
EXTERN(__CORE_INTERRUPTS);
EXTERN(__EXTERNAL_INTERRUPTS);
EXTERN(__EXCEPTIONS);
EXTERN(_dispatch_core_interrupt);
EXTERN(_dispatch_external_interrupt);
EXTERN(_dispatch_exception);


/* Default exception handler. By default, the exception handler is abort.
   Users can override this alias by defining the symbol themselves */
PROVIDE(ExceptionHandler = abort);

/* Default interrupt handler. By default, the interrupt handler is abort.
   Users can override this alias by defining the symbol themselves */
PROVIDE(DefaultHandler = abort);

/* The following symbols may be overriden in user code */
PROVIDE(Breakpoint = abort);

/* Core interrupt sources. */
PROVIDE(MachineExternal = DefaultMachineExternalHandler);
PROVIDE(MachineSoft = DefaultHandler);
PROVIDE(MachineTimer = DefaultHandler);
PROVIDE(SupervisorExternal = DefaultHandler);
PROVIDE(SupervisorSoft = DefaultHandler);
PROVIDE(SupervisorTimer = DefaultHandler);

/* External interrupt sources */
PROVIDE(SensInterrupt = DefaultHandler);
PROVIDE(GpioInterrupt = DefaultHandler);

/* Exception sources */
PROVIDE(IllegalInstruction = ExceptionHandler);
PROVIDE(InstructionFault = ExceptionHandler);
PROVIDE(InstructionMisaligned = ExceptionHandler);
PROVIDE(LoadFault = ExceptionHandler);
PROVIDE(LoadMisaligned = ExceptionHandler);
PROVIDE(StoreFault = ExceptionHandler);
PROVIDE(StoreMisaligned = ExceptionHandler);

PROVIDE(MachineEnvCall = ExceptionHandler);
PROVIDE(SupervisorEnvCall = ExceptionHandler);
PROVIDE(UserEnvCall = ExceptionHandler);

PROVIDE(InstructionPageFault = ExceptionHandler);
PROVIDE(LoadPageFault = ExceptionHandler);
PROVIDE(StorePageFault = ExceptionHandler);
