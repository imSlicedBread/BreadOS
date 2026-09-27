ENTRY(kernel_entry)
SECTIONS
{
  . = 0x200000;
  .text : { *(.text .text.*) }
  . = ALIGN(4096);
  .rodata : { *(.rodata .rodata.*) }
  . = ALIGN(4096);
  .got : { *(.got .got.*) }
  . = ALIGN(4096);
  .data : { *(.data .data.*) }
  . = ALIGN(4096);
  .bss : { *(COMMON) *(.bss .bss.*) }
  /DISCARD/ : { *(.eh_frame .eh_frame_hdr .comment .note .note.*) }
}
