/*
 * SPDX-FileCopyrightText: 2022 Espressif Systems (Shanghai) CO LTD
 *
 * SPDX-License-Identifier: Apache-2.0
 */

INCLUDE interrupts.x

ENTRY(reset_vector)

CONFIG_ULP_COPROC_RESERVE_MEM = 8 * 1024;

MEMORY
{
    ram(RW) : ORIGIN = 0, LENGTH = CONFIG_ULP_COPROC_RESERVE_MEM
}

REGION_ALIAS("REGION_TEXT", ram);
REGION_ALIAS("REGION_RODATA", ram);
REGION_ALIAS("REGION_DATA", ram);
REGION_ALIAS("REGION_BSS", ram);
REGION_ALIAS("REGION_HEAP", ram);
REGION_ALIAS("REGION_STACK", ram);

PROVIDE(_heap_size = 0);
PROVIDE(_stack_start = ORIGIN(ram) + LENGTH(ram));

SECTIONS
{
  . = ORIGIN(ram);


/DISCARD/ :
{
  *(.init)           /* Discard the riscv-rt provided _start function */
  *(.init.rust)      /* Discard the riscv-rt provided _start_rust function */
  *(.trap.rust);     /* Discard riscv-rt provided '_start_trap_rust' Rust function */
  *(.trap.vector);   /* Discard riscv-rt provided _trap_vector (vectored mode only) */
  *(.trap.start);    /* for _start_trap routine */
  *(.trap.start.*);  /* for _start_INTERRUPT_trap routines (vectored mode only) */
  *(.trap.continue); /* for _continue_trap routine (vectored mode only) */
  *(.trap .trap.*);  /* Other .trap symbols at the end */
}

  .text :
  {
    /* Power-on-reset must be placed at address 0x0 */
    KEEP(*(.ulp_reset));

    /* ULP will jump to 0x10 when an interrupt trap occurs.
     * This is where we must place the trap vector ASM.
     */
    . = 0x10;
    KEEP(*(.ulp_trap));       /* ULP traps will jump to 0x10 */
    KEEP(*(.ulp_init.start)); /* ulp_reset will jump to _ulp_start on reset/boot */
    KEEP(*(.ulp_trap.rust));  /* Rust trap handlers */
    KEEP(*(.ulp_init));       
    KEEP(*(.ulp_init.rust));
    *(.text.abort);
    *(.text .text.*);
    . = ALIGN(4);
  } >ram

  .rodata ALIGN(4):
  {
    *(.rodata)
    *(.rodata*)
  } >ram

  /* .data will not be initialised on _start,
   * so that we can use it as non-volatile memory.
   * This only works if ADDR(.data) == LOADADDR(.data),
   * which is true for the ULP core.
   */
  .data ALIGN(4):
  {
    __sdata = .; 
    PROVIDE(__global_pointer$ = . + 0x800);
    *(.data)
    *(.data*)
    *(.sdata)
    *(.sdata*)
    __edata = .;
  } >ram

  /* Address of .data in ROM memory */
  __sidata = LOADADDR(.data);

  /* .bss will be zero-ed every boot */
  .bss ALIGN(4) :
  {
    __sbss = .;
    *(.bss)
    *(.bss*)
    *(.sbss)
    *(.sbss*)
    __ebss = .;
  } >ram

  /* fictitious region that represents the memory available for the heap */
  .heap (NOLOAD) : ALIGN(4)
  {
    __sheap = .;
    . += _heap_size;
    . = ALIGN(4);
    __eheap = .;
  } > REGION_HEAP

  /* fictitious region that represents the memory available for the stack */
  .stack (NOLOAD) :
  {
    __estack = .;
    . = ABSOLUTE(_stack_start);
    __sstack = .;
  } > REGION_STACK
  
  /* fake output .got section */
  /* Dynamic relocations are unsupported. This section is only used to detect
     relocatable code in the input files and raise an error if relocatable code
     is found */
  .got (INFO) :
  {
    KEEP(*(.got .got.*));
  }
}
