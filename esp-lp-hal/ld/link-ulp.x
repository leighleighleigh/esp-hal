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
    KEEP(*(.init))      /* Discard the riscv-rt provided _start function */
    KEEP(*(.trap.rust)) /* Discard the original _start_trap_rust function */
  }

  .text :
  {
    /* Power-on-reset must be placed at address 0x0 */
    KEEP(*(.reset));
    /* ULP will jump to 0x10 when an interrupt trap occurs */
    . = 0x10;
    KEEP(*(.trap));
    /* KEEP(*(.init)); */
    KEEP(*(.init.rust));
    /* KEEP(*(.trap.rust)); */
    *(.text .text.*)
  } >ram

  .rodata ALIGN(4):
  {
    *(.rodata)
    *(.rodata*)
  } >ram

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

  .bss ALIGN(4):
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
