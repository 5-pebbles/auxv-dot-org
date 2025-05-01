<head>
  <title>Frankenstein's Monster | Auxv.org</title>
  <meta name="author" content="Owen Friedman">
<head/>

# Frankenstein's Monster 🧟

The executables that run on your operating system are often explained away as machine code: raw instructions your computer can understand. The truth is they are more of a Frankenstein's monster stored in a format called `Elf` (🧝), which stands for executable and linkable format. The contents of `Elf` files are split into sections. Those labeled `PT_LOAD` are just raw instructions, but others contain information such as the addresses and names of the functions.

<br/>

Just like in high-level programming languages, the `Elf` format uses libraries and imports. The names of these libraries are stored in the `PT_DYNAMIC` section and are referred to as shared objects. When the program starts, the dynamic linker finds these dependencies and loads them from file into the same address space as the main program. Then, using another set of data structures in the `PT_DYNAMIC` section, called relocations, it updates all calls to these functions with their new addresses.

<br/>
<details>
<summary><b>Table of Contents:</b></summary>

- [Frankenstein's Monster 🧟](#frankensteins-monster)
- [Magic Numbers 🪄1️⃣2️⃣3️⃣](#magic-numbers-123)
  - [Elf Header 🧝⚔️🧌](#elf-header)
  - [Program Header Table `(╯°□°)╯︵ ┻━┻`](#program-header-table)
- [Vaguely Related Tangential Discussion of `x86_64` 🪟🐿️](#vaguely-related-tangential-discussion-of)
- [Some Assembly Required 🧰](#some-assembly-required)
- [It's Alive ⚡](#its-alive)

</details>


## Magic Numbers 🪄1️⃣2️⃣3️⃣

A [magic number](https://en.wikipedia.org/wiki/List_of_file_signatures), or file signature, is a sequence of bytes used to identify a file format. Kind of like how a file extension helps <u>_**you**_</u> identify the file format, a signature helps the operating system and other applications do the same. A key difference is that it works regardless of the name and extension assigned by the file system.

<br/>

A good example of this is the shebang used by shell scripts (`#!`), which tells the operating system to pass the script path as an argument to the program immediately following (e.g. `/bin/sh`).

### Elf Header 🧝⚔️🧌

The magic number for `Elf` files is the bytes [`7f`, `45`, `4c`, `46`] or [`0x7F`, `'E'`, `'L'`, `'F'`]. This will tell the operating system to read the first bytes of the file as an Elf header, defined in the [Elf specification](https://refspecs.linuxfoundation.org/elf/elf.pdf) as this data structure:

```rs
/// A file with an unknown type.
pub const ET_NONE: u16 = 0;
/// A relocatable object file (.a and .o file extentions).
pub const ET_REL: u16 = 1;
/// An executable file.
pub const ET_EXEC: u16 = 2;
/// A dynamic library/object file (.so extention).
pub const ET_DYN: u16 = 3;
/// This is used for core dump files (IDK)...
pub const ET_CORE: u16 = 4;

#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq)]
pub struct ElfHeader {
    /// ELF identification array, containing the magic number and other information.
    ///
    /// The layout is as follows:
    /// - [0..4]: Magic Number (0x7F, 'E', 'L', 'F')
    /// - [4]: File Class (1 = 32-bit, 2 = 64-bit)
    /// - [5]: Endianness (1 = little-endian, 2 = big-endian)
    /// - [6]: Elf Version (should be 1)
    /// - [7]: OS ABI (0 = System V, 3 = Linux, etc.)
    /// - [8]: ABI Version
    /// - [9..16]: Padding (currently unused)
    pub e_ident: [u8; 16],
    /// The Elf file type, see the ET_.* constants.
    pub e_type: u16,
    /// The target archectecture, see the TODO constants.
    pub e_machine: u16,
    /// The Elf format version, only version one is currently available.
    pub e_version: u32,
    /// The virtual address to which the kernal or dynamic linker will jump when begining execution.
    pub e_entry: usize,
    /// The offset into the file at which the program header table resides.
    pub e_phoff: usize,
    /// The offset into the file at which the section header table resides.
    pub e_shoff: usize,
    /// A collection of processor-specific flags.
    pub e_flags: u32,
    /// The size of the Elf header you are currently reading, 52 for 32-bit systems and 64 for 64-bit ones.
    pub e_ehsize: u16,
    /// The size of each Elf program header table entry in bytes.
    pub e_phentsize: u16,
    /// The number of Elf program header table entries.
    pub e_phnum: u16,
    /// The size of each Elf section header table entry in bytes.
    pub e_shentsize: u16,
    /// The number of Elf section header table entries.
    pub e_shnum: u16,
    /// The index into the section header table at which the string table resides.
    pub e_shstrndx: u16,
}
```

### Program Header Table `(╯°□°)╯︵ ┻━┻`

In the very beginning of this article, I mentioned `PT_LOAD` sections; which are stored in the program header table. While they should have a corresponding entry in the section header table for use in debugging and static/compile time linking, I am ignoring that requirement for now because it's not strictly necessary.

<br/>

At runtime, the first bytes are read, and the file is identified as an `Elf`. Then, the Elf header is parsed according to the structure above. Using the file offset (`e_phoff`), number of entries (`e_phnum`), and corresponding entry size (`e_phentsize`), the program header table is parsed as an array of the following data structure:

```rs
/// A loadable segment.
pub const PT_LOAD: u32 = 1;
/// Dynamic linking information.
pub const PT_DYNAMIC: u32 = 2;
/// Program header table information.
pub const PT_PHDR: u32 = 6;
/// Thread-local storage template.
pub const PT_TLS: u32 = 7;

#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq)]
pub struct ProgramHeader {
    /// Identifies the type of the segment, see the PT_.* constants.
    pub p_type: u32,
    /// A collection of segment-specific flags (e.g. execute, read, write permissions).
    /// NOTE: This fields position is dependent on target point width...
    #[cfg(target_pointer_width = "64")]
    pub p_flags: u32,
    /// The offset into the file at which the segment resides.
    pub p_offset: usize,
    /// The virtual address at which the kernel or dynamic linker will load the segment.
    pub p_vaddr: usize,
    /// The physical address at which the previous virtual address is mapped (not used on x86_64).
    pub p_paddr: usize,
    /// The size of the segment in file.
    pub p_filesz: usize,
    /// The size of the segment in runtime memory.
    pub p_memsz: usize,
    /// A collection of segment-specific flags (e.g. execute, read, write permissions).
    /// NOTE: This fields position is dependent on target point width...
    #[cfg(target_pointer_width = "32")]
    pub p_flags: u32,
    /// Alignment of the segment, must be a power of 2 (0 or 1 means no alignment is required).
    pub p_align: usize,
}
```

In this article, we will write a simple `Elf` file by hand. In this example there will be one program header of type `PT_LOAD`. The operating system will `mmap` the section to the virtual address specified in the program header and the jump to the entry address specified in the Elf header.

> If your curious about the smallest Elf "hello world" look at [this](https://nathanotterness.com/2021/10/tiny_elf_modernized.html) blog post by Nathan Otterness.


## Vaguely Related Tangential Discussion of `x86_64` 🪟🐿️

Alright, so if we are going to write an `Elf` file by hand, using raw `x86_64` assembly makes the most sense. Usually, the assembler (`nasm` in my case) automatically generates the `Elf` related data structures; we can negate this with the `-f bin` command line argument, which tells the assembler to only include the code we explicitly write.

<br/>

And thus begins the stupidity: any modern code will fail to compile with this flag, raising errors like `instruction not supported in 16-bit mode`, because `x86_64` is technically a 16-bit architecture by default. This is to avoid breaking backwards compatibility with code written before 1985, when Intel released the 80386 32-bit processor.

<br/>

Typically, you don't have to worry about this <u>_**retarded bull shit**_</u>, because the bootloader steps through 16-bit mode, enables the [A20 line](https://en.wikipedia.org/wiki/A20_line), then through 32-bit mode, while simultaneously jumping back and forth between [real mode](https://en.wikipedia.org/wiki/Real_mode) and [protected mode](https://en.wikipedia.org/wiki/Protected_mode), before finally arriving at 64-bit mode and passing control to the desired kernel.

<br/>

This is also why an `x86_64` [word](https://en.wikipedia.org/wiki/Word_(computer_architecture)) is still only 16-bits <strong id="yearsSince"></strong> years after those systems became obsolete... You don't really need to know any of this, but I think it's an important part of computer history. The solution is to annotate your script as 64-bit using the `[BITS 64]` assembler directive.

<script>
  document.getElementById("yearsSince").innerHTML = new Date().getFullYear() - 1985;
</script>

Anyway, let's write some real code... 👋


## Some Assembly Required 🧰

We know our code needs to start with the bits 64 directive:

```x86asm
[BITS 64]
```

And we need to designate a virtual address, at which the kernel will `mmap` this file. Said address can be bound to a symbol using the `equ` instruction:

```x86asm
virtual_address: equ 4096 * 20 ; I just picked 20 pages arbitrarily, but it can't be too low, because some of those address are reserved.
```

Then, we can use the `db` (define byte) family of instructions to define the Elf header and program header table:

| Instruction | Meaning | Bits |
|:-----------:|:-------:|:----:|
| `db` | Define Byte  | 8-bits | 
| `dw` | Define Word | 16-bits |
| `dd` | Define Double | 32-bits |
| `dq` | Define Quad | 64-bits |

```x86asm
elf_header:
    ; Magic Number Identifing The Elf Format:
    db 0x7F, 'E', 'L', 'F'
    ; Elf Class (64-bit):
    db 2
    ; Endianness (litte endian):
    db 1
    ; Elf Version (1):
    db 1
    ; Operating System ABI and the rest of e_ident which we don't use:
    db 0, 0, 0, 0, 0, 0, 0, 0, 0
    ; File type (executable):
    dw 2
    ; Instruction Set Architecture (amd x86_64):
    dw 0x3E
    ; Another Elf Version Field (IDK, I am just gonna set it to 1):
    dd 1
    ; Entry point which the kernal will jump execution to:
    dq virtual_address + x86_64_code
    ; Address of the program headers:
    dq program_header_table
    ; Address of the section headers which we are not implementing:
    dq 0
    ; A bunch of flags that I'm not using:
    dd 0
    ; Size of this Elf Header (64 bytes):
    dw 64
    ; Size of a Program Header (56 bytes):
    dw 56
    ; Number of Program Headers:
    dw 1
    ; Size of a Section Header (64 bytes):
    dw 64
    ; Number of Section Headers:
    dw 0
    ; The Section Header index of the string table (not used):
    dw 0


; We only need one program header, it will be a PT_LOAD containing the whole file.
program_header_table:
    ; Segment Type (PT_LOAD):
    dd 1
    ; Flags With The Following Bit-Positions
    ; Executable=0x1
    ; Writeable=0x2
    ; Readable=0x4
    ; (Read and Execute):
    dd 0b101
    ; The Offset Into The File At Which The Segment Resides:
    dq 0
    ; The Virtual Address At Which To Load The Segment:
    dq virtual_address
    ; The Physical Address At Which The Previous Virtual Address Is Mapped (not used on x86_64):
    dq virtual_address
    ; The Size Of The Segment In The File (last_address - first_address or in this case 0):
    dq last_address
    ; The Size Of The Segment In Memory (same as above):
    dq last_address
    ; Required Alignment Of The Segment (the size of a x86_64 standard page):
    dq 0x1000
```

All that remains is to write `"Hello World!\n"` to `stdout` and exit the process with a status code of **0**. Thankfully, we are still running on top of an operating system, which means we have a standard library of sorts.

<br/>

When the kernel assumes control from the bootloader, it registers a function with the CPU. On 32-bit systems this function is triggered by a `0x80` interrupt, and on 64-bit systems there is a dedicated `syscall` instruction, but the idea remains the same. The function uses the value in the `rax` register to delegate to a system call handler like [`mmap`, `write`, or `exit`].

<br/>

The calling convention is similar to that of the **C** ABI and resides within the same [specification](https://refspecs.linuxbase.org/elf/x86_64-abi-0.99.pdf#subsection.A.2.1). In the following code I will use two system calls: one to write to `stdout` (file descriptor #1) and another to exit the process. Lastly, I will append the `last_address` label, so the `PT_LOAD` section contains the whole file:

```x86asm
; This label defines our code and is pointed to by the entry in the elf header.
x86_64_code:
    ; Write Syscall:
    mov rax, 1                             ; Syscall Index
    mov rdi, 1                             ; File Descriptor
    mov rsi, virtual_address + hello_world ; String Slice Pointer
    mov rdx, hello_world_length            ; String Length
    syscall

    ; Exit Syscall:
    mov rax, 60 ; Syscall Index
    mov rdi, 0  ; Exit Code
    syscall

    ud2 ; This is an invalid instruction; it's used as a safety barrier incase the exit ever fails...


hello_world: db `Hello World!\n`
hello_world_length: equ $ - hello_world

last_address:
```

Another side effect of `x86_64`'s extreme backwards compatibility is that we can actually use the same registers they used back in the early 1980's:

| Name  |    Bits   | Meaning |
|:-----:|:---------:|:-------:|
| `al`  | bits 0-7  | Accumulator low |
| `ah`  | bits 8-15 | Accumulator high (the sequel) |
| `ax`  | bits 0-15 | Accumulator eXtended (marketing department got involved) |
| `eax` | bits 0-31 | Extended Accumulator eXtended (we heard you like extensions) |
| `rax` | bits 0-63 | Register Accumulator eXtended (running out of letters here) |

> <b style="color: var(--love);">Hot take:</b> They really dropped the ball on naming the 64-bit registers. It should've been xeax (eXtended Extended Accumulator eXtended) for maximum confusion. The 128-bit register could still be exeax though...

<br/>

These just operate as a subsection of the corresponding 64-bit registers, but it allows you to, for example use the 32-bit system calls:
<!-- Kate doesn't like this part... -->

```x86asm
x86_32_code:
    ; Write Syscall:
    mov eax, 4                             ; Syscall Index for write in 32-bit
    mov ebx, 1                             ; File Descriptor
    mov ecx, virtual_address + hello_world ; String Slice Pointer
    mov edx, hello_world_length            ; String Length
    int 0x80                               ; Invoke syscall via interrupt 0x80
    
    ; Exit Syscall:
    mov eax, 1 ; Syscall Index for exit in 32-bit
    mov ebx, 0 ; Exit Code
    int 0x80   ; Invoke syscall via interrupt 0x80
    
    ud2


hello_world: db `Hello World!\n`
hello_world_length: equ $ - hello_world

last_address:
```

I don't know why anyone would ever want to do this, but you can, and it even saves a few bytes of space... if you really need it.


## It's Alive ⚡

All that's left to do is compile and run the code; I'm lazy, so here's a script:

```sh
#!/bin/sh
nasm -f bin -o elf_by_hand elf_by_hand.asm
chmod +x elf_by_hand
./elf_by_hand
```

And ta-da:

```
❯ ./build.sh
Hello World!
❯ echo $? # The environment variable ? is the exit status of the last command...
0
```

If we want a byte by byte hex dump of the binary, we can use `hexdump -X ./elf_by_hand`, and with some ascii art + 2 hours of debugging why font ligatures weren't working on your website:
<!-- Kate doesn't like that part either... -->

```x86asm
; ┌-------------------------------------------------------------------------------┐
; │ 7f   45   4c   46   02   01   01   00   00   00   00   00   00   00   00   00 │ <---┐
; │ 02   00   3e   00   01   00   00   00   78   40   01   00   00   00   00   00 │     │
; │ 40   00   00   00   00   00   00   00   00   00   00   00   00   00   00   00 │     │
; │                                                                          ┌----┤     │
; │ 00   00   00   00   40   00   38   00   01   00   40   00   00   00   00 │ 00 │ <--┐│
; ├--------------------------------------------------------------------------┘    │    ││
; │ 01   00   00   00   05   00   00   00   00   00   00   00   00   00   00   00 │    ││
; │ 00   40   01   00   00   00   00   00   00   40   01   00   00   00   00   00 │    ││
; │ ae   00   00   00   00   00   00   00   ae   00   00   00   00   00   00   00 │    ││
; │                                       ┌---------------------------------------┤    ││
; │ 00   10   00   00   00   00   00   00 │ b8   01   00   00   00   bf   01   00 │ <-┐││
; ├---------------------------------------┘                                       │   │││
; │ 00   00   48   be   a1   40   01   00   00   00   00   00   ba   0d   00   00 │   │││
; │ 00   0f   05   b8   3c   00   00   00   bf   00   00   00   00   0f   05   0f │   │││
; │    ┌----------------------------------------------------------------┬---------┘   │││
; │ 0b │ 48   65   6c   6c   6f   20   57   6f   72   6c   64   21   0a │ <----┐      │││
; └----┴----------------------------------------------------------------┘      │      │││
;                                                                              │      │││
; Elf Header    Program Header Table    x86_64 code    Hello World!\n ---------┘      │││
;    │                    │                  │                                        │││
;    │                    │                  └----------------------------------------┘││
;    │                    └------------------------------------------------------------┘│
;    └----------------------------------------------------------------------------------┘
```

And with that you have a handwritten `Elf` file, in its beautiful simplicity.

<br/>

That's all for today folks!

<!-- [Next: Chapter 2](/projects/miros/the_three_musketeers) -->
