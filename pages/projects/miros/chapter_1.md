# Chapter 1: Where to `_start`

As part of my ongoing campaign to convince the world I am worthy of an entry level position, I am writing a dynamic linker from scratch in **Rust**.

<br/>

It's turning out to be a lot harder than I thought (go figure), but that's mostly due to a lack of documentation.
This series is my attempt to fix that, it's solely aimed at Linux because I have no idea how Windows works...


## What's A Dynamic Linker?

The binaries on your computer aren't just raw instructions, they are stored in a format called `Elf` which stands for executable linking format. The contents of `Elf` files are split into sections. Those labeled `PT_LOAD` are just raw instructions, but others contain information such as the addresses and names of the functions. 

<br/>

Just like in high-level programming languages, the `Elf` format uses libraries and imports. The names of these so-called shared libraries are stored in the `PT_DYNAMIC` section and are referred to as shared objects. When the program starts, the dynamic linker finds these dependencies and loads them from a file into the same address space as the main program. Then, using another set of data structures in the `PT_DYNAMIC` section called relocations, it updates all calls to these functions with their new addresses.

<br/>

While statically linked binaries have become more popular lately, they too have relocations requiring a dynamic linker to resolve at runtime. This is part of a system called `ASLR`, which stands for address space layout randomization. `ASLR` randomly arrange the address space including the positions of the base executable, stack, heap, and any shared objects. Then, the dynamic linker updates all addresses using the base address (B) plus an offset into the executable called the addend (A). This allows the executable to locate its own functions and variables.

<br/>

As far as I know, the only alternatives to this is a program that acts as its own dynamic linker. A good example of this is the **Rust** crate [origin](https://github.com/sunfishcode/origin), which can relocate its own statically linked executable.


## Now Let's Begin

As you can guess, the dynamic linker is one of those executables that relocates itself. Actually, the default dynamic linker `ld.so` is completely dynamic, linking its own libraries at runtime. I am not going to be doing that; it's a lot of work and **Rust** crates are always statically linked, anyway.

<br/>

If you have done any low-level work, you are familiar with the `_start` symbol. It's part of `crt0` or **C** runtime zero. In a normal **Rust** program it is the first code to run:

```
// [_start:c] -> [main:c] -> [start:rust] -> [main:rust] <┐
//                                                        |
// This is the function a developer writes. --------------┘
```

The symbol `_start` is in fact arbitrary, the actual entry point is defined in the `Elf` headers `e_entry` field. The dynamic linker, or the Linux kernel (in cases without a linker), will jump to that address to start execution.

You can view the address of the entry point using the `readelf` command with the `-h` argument:
```
❯ readelf -h ./target/debug/example
ELF Header:
  Magic:   7f 45 4c 46 02 01 01 00 00 00 00 00 00 00 00 00 
  Class:                             ELF64
  Data:                              2's complement, little endian
  Version:                           1 (current)
  OS/ABI:                            UNIX - System V
  ABI Version:                       0
  Type:                              DYN (Position-Independent Executable file)
  Machine:                           Advanced Micro Devices X86-64
  Version:                           0x1
  Entry point address:               0x14da0
  Start of program headers:          64 (bytes into file)
  Start of section headers:          3971152 (bytes into file)
  Flags:                             0x0
  Size of this header:               64 (bytes)
  Size of program headers:           56 (bytes)
  Number of program headers:         12
  Size of section headers:           64 (bytes)
  Number of section headers:         43
  Section header string table index: 41
```

We can overwrite the default `_start` function with our own, by disabling `crt0` with `rustc`s `-C link-arg=-nostartfiles` argument. You can automatically add it to every cargo build with a `.cargo/config.toml`:

```toml
[build]
rustflags = ["-C", "link-arg=-nostartfiles", "-C", "target-feature=+crt-static"]
```

We also need to add the `-C target-feature=+crt-static` flag to enable static linking of the **C** runtime (crt0 without the zero).

Now we need to define our own `_start` in `src/main.rs`:

```rs
#![feature(naked_functions)]
#![no_main]

use std::arch::naked_asm;

#[naked]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _start() -> ! {
    naked_asm!("mov rdi, rsp",
        "and rsp, -16", // !0b1111
        "call {}",
        "jmp rax",
        sym relocate_and_calculate_jump_address,
    );
}
```

This creates a label `_start`, which calls the function `relocate_and_calculate_jump_address` with stack pointer as an argument. Then, it jumps to the address returned by that function.
It requires some fancy features only available in the night version of rust, which you can enable for the current project using the following command: `rustup override set nightly`.

<br/>

The `#![feature(naked_functions)]` enables a nightly feature for your project and should be placed at the top of `src/main.rs`. 
A naked function disables the usual prologue/epilogue, leaving argument and return value handling to the developer. All naked functions must be marked `unsafe` and `extern "C"`. They only include the Assembly defined within the `naked_asm` macro. We will get to the `extern "C"` part later.

<br/>

The `#![no_main]` attribute macro tells the compiler that, yes, we know what we are doing and didn't just forget to define a main function.

<br/>

A function like `main` would normally be **mangled** to something like `_ZN5miros4main17h7f8645e8adf41fc7E`, to avoid conflicts while linking together crates. The attribute macro `#[no_mangle]` tells the compiler to let the identifier maintain its original name.

<br/>

_**NOTE:**_ I prefer intel syntax `<instruction> <destination>, <source>`, but you can also use AT&T syntax by using the `att_syntax` option.

**Intel:**
```rs
naked_asm!("movq rdi, rsp",
    "and rsp, -16", // !0b1111
    "call {}",
    "jmp rax",
    sym relocate_and_calculate_jump_address,
);
```

**AT&T:**
```rs
naked_asm!("movq %rsp, %rdi",
    "andq $~15, %rsp", // !0b1111
    "callq {}",
    "jmpq *%rax",
    sym relocate_and_calculate_jump_address,
    options(att_syntax),
);
```


## Abstract Binary Interface

An ABI, or abstract binary interface, defines how program modules and functions communicate. I have two devices, and both fall under the [System V ABI - AMD64 Architecture Processor Supplement](https://refspecs.linuxbase.org/elf/x86_64-abi-0.99.pdf).

<br/>

While I recommend reading the entire ABI for your architecture (a few times), the only important parts here (at least on `x86_64`) are:
1. Integer arguments are passed in the `rdi`, `rsi`, `rdx`, `rcx`, `r8` and `r9` registers.
2. The stack must be 16-byte aligned. If you forget this part, like I did, your program will segfault on any misaligned `movaps`.
3. If two or fewer integers are returned, they are passed in the `rax` and `rdx` registers.


<br/>

We can tell **Rust** code to follow an ABI using the `extern` keyword. For example, defining a function with `extern "C"` means it will use the ABI we just described.

<br/>

We can use this to define `relocate_and_calculate_jump_address`:
```rs
pub unsafe extern "C" fn relocate_and_calculate_jump_address(stack_pointer: *mut usize) -> usize {
    0
}
```

This function will assume the stack is 16-byte aligned and expect its first argument to be in `rdi`. It will then return a `usize` via the `rax` register.

Now we have called **Rust** code from our `_start`... And yes, this will of course segfault, when `_start` jumps to 0. But, that can be fixed in the next chapter.

<br/>

(To be continued)
