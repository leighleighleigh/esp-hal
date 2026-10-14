/*       Enable compressed instructions */
.option  rvc

/* Much of the following assembly was sourced from the following ESP-IDF files...
*  ...irq handler macros:
*    https://github.com/espressif/esp-idf/blob/12f36a021f511cd4de41d3fffff146c5336ac1e7/components/ulp/ulp_riscv/ulp_core/ulp_riscv_vectors.S
*  ...critical section assembly
*    https://github.com/espressif/esp-idf/blob/12f36a021f511cd4de41d3fffff146c5336ac1e7/components/ulp/ulp_riscv/ulp_core/include/ulp_riscv_utils.h
*  ...riscv halt code
*    https://github.com/espressif/esp-idf/blob/12f36a021f511cd4de41d3fffff146c5336ac1e7/components/ulp/ulp_riscv/ulp_core/ulp_riscv_utils.c
*/

.section .ulp_reset, "ax"
.global ulp_reset_vector 
/* Power-on reset vector, placed at address 0x0 */
ulp_reset_vector:
	/* no more than 16 bytes here */
	jal zero, _ulp_start

/* Hardware trap vector, placed at address 0x10
 * Handles all interrupts and exceptions for the ULP core.
 * Saves registers ra,t0-6,a0-7, then calls _start_trap_rust.
 */
.section .ulp_trap, "ax"
.global _ulp_start_trap
_ulp_start_trap:
    /* Save the general gurpose register context before handling the interrupt */
    save_general_regs

    /* Store the return address (x1) and stack pointer (x2) in the q registers. */
    setq_insn q2, ra
    setq_insn q3, sp

    /* Store the pointer to the stored registers (TrapFrame) in a0.
     * This will be the first argument to _start_trap_rust.
     */
    addi a0, sp, 0

    /* Fetch the interrupt status from the custom q1 register into a1.
     * This will be the second argumet to _start_trap_rust.
     */
    getq_insn a1, q1
    /* Call the global interrupt handler. The interrupt status is passed as the argument in a0.
     * We do not re-enable interrupts before calling the handler as ULP RISC-V does not
     * support nested interrupts.
     * The normal _start_trap_rust, provided by riscv-rt crate, is /DISCARD/-ed.
     */
    jal ra, _ulp_start_trap_rust

    /* Restore the return address and stack pointer from the q registers */
    getq_insn ra, q2
    getq_insn sp, q3

    /* Restore the register context after returning from the C interrupt handler */
    restore_general_regs

    /* Exit interrupt handler by executing the custom retirq instruction which will restore pc and re-enable interrupts */
    retirq_insn

/* Initialisation routine */
.section .ulp_init.start, "ax"
.global _ulp_start
_ulp_start:
    /* Setup the global pointer & thread pointer */
    .option push
    .option norelax
    la gp, __global_pointer$
    addi tp, gp, 0
    .option pop
    /* Setup the stack pointer & frame pointer */
    la sp, _stack_start
    add s0, sp, zero
    /* Jump to rust startup code */
    jal zero, _ulp_start_rust
