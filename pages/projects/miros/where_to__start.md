<head>
  <title>Where to _start? | Auxv.org</title>
  <meta name="author" content="Owen Friedman">
<head/>

# Where to `_start`?

The goal of this project is to write a dynamic linker in Rust, well that's kind of a lie, I'm just playing around. Still...

<br/>

Like high-level programming languages, the `Elf` format uses libraries and imports. The names of these libraries (shared objects) are stored in the `PT_DYNAMIC` section. When the program starts, the dynamic linker/loader finds these dependencies and loads them from file into the same address space as the main program. Using a list of pointer equation pairs in the `PT_DYNAMIC` section called relocations it updates all calls to these functions with their new addresses.

<br/>

As far as I know, the only alternatives to this are programs acting as their own dynamic linker or static linked programs with a predefined address in memory (no [PIC](https://en.wikipedia.org/wiki/Position-independent_code) position-independent code / can't use [ASLR](https://en.wikipedia.org/wiki/Address_space_layout_randomization)). A couple good examples of the former are the Rust crate [origin](https://github.com/sunfishcode/origin), [musl-libc](https://musl.libc.org/)'s rcrt1.o, and the [zig](https://ziglang.org/) programming language's runtime, which can all relocate themselves.

<br/>
<details>
<summary><b>Table of Contents:</b></summary>

- [Where to `_start`?](#where-to)
  - [The Illusion of Separation 🎩🪄](#the-illusion-of-separation)
  - [Now Let's Begin 🍫](#now-lets-begin)
    - [The Secret Sauce: What's on the Stack? 🥞](#the-secret-sauce-whats-on-the-stack)
    - [Gettin' the Goods 🍟](#gettin-the-goods)
      - [Application Binary Interface 0⃣1⃣0⃣1⃣](#abstract-binary-interface-0101)
      - [Inline Assembly 📏🪛🪚](#inline-assembly)
  - [Home Sweet Home 🐚🦀](#home-sweet-home)

</details>

## The Illusion of Separation 🎩🪄

For now, we need to flesh out our own runtime. Without `ld.so`, the entire C standard library ceases to function. The standard library and pthreads (in both musl and glibc) are heavily dependent on their dynamic linker's specific initialization. A compatible linker would have to initialize literally hundreds of <u>**undocumented and unstable**</u> global variables, structs, and fields.

<br/>

At this point, both dynamic linkers are really just components of their corresponding standard libraries. This is why the [dryad](https://github.com/m4b/dryad) project was abandoned: you can't write a drop-in `ld.so` replacement without also controlling the standard library. They depend on each other's internal behavior, not just public interfaces.

<br/>

The current "separation" is a lie. They are deeply coupled, just dishonestly so. Some functionality, like thread-local storage and `pthread_cancel` coordination, can't be cleanly separated between the dynamic linker/loader, the standard library, and pthreads. As [noted](https://github.com/m4b/dryad/issues/5#issuecomment-262696880) by m4b, there are structs like `rtld_global_ro` who's definitions would need to be duplicated and populated in order to integrate with glibc's dynamic dispatch, and resolver functions.

> <b style="color: var(--foam)">Note:</b> That struct definition has honestly gotten worse in the 9 years since m4b commented. There are now `#include<somepackage>` statements in the definition. If you don't understand [read this](https://en.wikipedia.org/wiki/Include_directive).

<br/>

There are two possible solutions (that I can think of):

1. **Define a clean API between the two:** This is hard to do. pthreads can't set up thread-local storage through something like an init array function because the dynamic linker needs to perform TLS relocations first. The dynamic linker is forced to mix responsibilities.

2. **Embed the C standard library and pthreads in the dynamic linker:** Accept that they are intertwined and make it explicit.

Given that I can only find 6 attempts to write a Linux dynamic linker in the last 30 years, and that [proliferating a new standard](https://xkcd.com/927/) used only by me would be pointless, I'll choose the embedding approach.


## Now Let's Begin 🍫


As you might guess, the dynamic linker is one of those executables that relocates itself. The default glibc dynamic linker `ld.so` is completely dynamic, linking its own libraries at runtime. Embedding our own copies of those libraries will cause most calls to these functions to be either inlined or called via [instruction pointer relative addressing](https://en.wikipedia.org/wiki/Addressing_mode#PC-relative) neither of which require relocation.

> <b style="color: var(--foam)">Note:</b> For those who don't know Rust crates are statically linked at compile time, so unless you import a crate linking to a c library, the only dynamic dependencies will be the standard library, pthreads, and gcc (gcc provides stack unwind, we'll implement that ourselves).

### The Secret Sauce: What's on the Stack? 🥞

The stack is conceptually an area of memory that is used for a first in last out data-structure. In `x86_64` Linux the top most address is stored in the `rsp` register. When your program starts the Linux kernel pushes three sub-data-structures onto the stack... command line arguments, environment variables, and the auxiliary vector.

1. **Command Line Arguments:** Are values passed to your program from the command line. For example if you run `git commit` there are two arguments `git` and `commit`.

2. **Environment Variables:** Are key-value pairs that provide context to the running program. They're commonly used for configuration and system information. For example, the string `HOME=/home/ghostbird` stores the path to your home directory.

3. **The Auxiliary Vector:** Is more obscure and this website's namesake. It's an assortment of possibly useful information passed form the Linux kernel to the dynamic linker/loader and/or runtime.

Here is a diagram to demonstrate the initial stack's layout in memory:

```x86asm
; + Newly Pushed Values      Example:                ┌-----------------┐
; ┌-------------------┐    ┌----------------┐  ┌---> | "/bin/git", 0x0 |
; | Arg Count         |    | 2              |  |     └-----------------┘
; |-------------------|    |----------------|  |
; | Arg Pointers...   |    | Pointer,       | -┘   ┌---------------┐
; |                   |    | Other Pointer  | ---> | "commit", 0x0 |
; |-------------------|    |----------------|      └---------------┘
; | Null              |    | 0x0            |
; |-------------------|    |----------------|       ┌-----------------------------┐
; | Env Pointers...   |    | Pointer,       | ----> | "HOME=/home/ghostbird", 0x0 |
; |                   |    | Other Pointer  | ---┐  └-----------------------------┘
; |-------------------|    |----------------|    |
; | Null              |    | 0x0            |    |   ┌---------------------------┐
; |-------------------|    |----------------|    └-> | "PATH=/bin:/usr/bin", 0x0 |
; | Auxv Type...      |    | AT_RANDOM      |        └---------------------------┘
; | Auxv Value...     |    | Union->Pointer | -┐
; |-------------------|    |----------------|  |   ┌---------------------------┐
; | AT_NULL Auxv Pair |    | AT_NULL (0x0)  |  └-> | [16-bytes of random data] |
; └-------------------┘    | Undefined      |      └---------------------------┘
;                          └----------------┘
```

On `x86_64` the stack grows down, and so we access each field by adding to the value stored in the `rsp` register, then dereferencing it as a pointer.

### Gettin' the Goods 🍟

If you have done any low-level runtime work, you are familiar with the `_start` symbol. It's part of `crt0` or C runtime zero. In a normal Rust program it is the first code to run:

```x86asm
; [_start:c] -> [main:c] -> [start:rust] -> [main:rust] <┐
;                                                        |
; This is the function a developer writes. --------------┘
```

> <b style="color: var(--gold);">Warning:</b> Yup, the Rust runtime is just staked on top of a C runtime and standard library.
>
> I personally think we should take a lesson from [zig](https://ziglang.org/) on this and use our own runtime by default.

<br/>

The symbol (`_start`) itself is arbitrary, the actual entry point is defined in the `Elf` headers `e_entry` field. The dynamic linker or Linux kernel will jump to that address to begin execution.

You can view the address of an entry point using the `readelf` command with the `-h` argument:
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

Passing through `crt0`, we'd lose the [initial stack pointer](https://refspecs.linuxbase.org/elf/x86_64-abi-0.99.pdf#section.3.4); so we need to define our own entry point.

#### Application Binary Interface 0⃣1⃣0⃣1⃣

An ABI (application binary interface) defines how data is structured in memory, how functions are called, and how registers are used on a machine code level. I have two devices, and both fall under the [System V ABI - AMD64 Architecture Processor Supplement](https://refspecs.linuxbase.org/elf/x86_64-abi-0.99.pdf).

<br/>

As part of most ABI's the compiler automatically inserts a prologue into the beginning of the function, storing state and setting up for the function body. This will modify the `rsp` register before we can access it and the initial stack state. The solution is a naked function, a naked function disables the usual prologue/epilogue, leaving argument and return value handling to the developer.

<br/>

I'd hate to write this whole application in assembly, so our `_start` function needs to call Rust code via a known ABI. The C ABI is the most universal and simple option, I recommend reading the entire ABI for your architecture (a few times). The important parts here (at least on `x86_64`) are:
1. Integer arguments are passed in the `rdi`, `rsi`, `rdx`, `rcx`, `r8` and `r9` registers.
2. The stack must be 16-byte aligned. If you forget this part, like I did, your program will segfault on any misaligned `movaps`.
3. If two or fewer integers are returned, they are passed in the `rax` and `rdx` registers.
4. The stack pointer is stored in the `rsp` register.


<br/>

We can tell Rust code to follow an ABI using the `extern` keyword. For example, defining a function with `extern "C"` means we'll use the ABI we just described.

#### Inline Assembly 📏🪛🪚

This code calls the C function `relocate_and_calculate_jump_address` with the stack 16-byte aligned, and the stack pointer as an argument. Lastly, it jumps to the address returned from that function:

```rs
#![no_main]

use std::arch::naked_asm;

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _start() -> ! {
    naked_asm!("mov rdi, rsp",
        "and rsp, -16", // !0b1111
        "call {}",
        "mov rdx, 0",
        "jmp rax",
        sym relocate_and_calculate_jump_address,
    );
}

```

The `#[unsafe(no_mangle)]` attribute macro stops the compiler from [mangling](https://en.wikipedia.org/wiki/Name_mangling) a function. E.g. `main` would become something like `_ZN5miros4main17h7f8645e8adf41fc7E` to avoid conflicts between functions with the same name, but differing scopes.


<br/>


As a side note, I prefer intel syntax `<instruction> <destination>, <source>`, but Rust supports AT&T syntax using the `att_syntax` option.

##### Intel
```rs
naked_asm!("movq rdi, rsp",
    "and rsp, -16", // !0b1111
    "call {}",
    "jmp rax",
    sym relocate_and_calculate_jump_address,
);
```

##### AT&T
```rs
naked_asm!("movq %rsp, %rdi",
    "andq $~15, %rsp", // !0b1111
    "callq {}",
    "jmpq *%rax",
    sym relocate_and_calculate_jump_address,
    options(att_syntax),
);
```

## Home Sweet Home 🐚🦀

Alright, with that we're done with assembly for a very short while. As mentioned previously we can define a Rust function conforming to the C ABI via `extern "C"`, in that function we can extract the command line arguments, environment variables, and the auxiliary vector. However, I'll leave parsing these structures for next time:

```rs
pub unsafe extern "C" fn relocate_and_calculate_jump_address(stack_pointer: *mut usize) -> usize {
    // + Newly Pushed Values      Example:                ┌-----------------┐
    // ┌-------------------┐    ┌----------------┐  ┌---> | "/bin/git", 0x0 |
    // | Arg Count         |    | 2              |  |     └-----------------┘
    // |-------------------|    |----------------|  |
    // | Arg Pointers...   |    | Pointer,       | -┘   ┌---------------┐
    // |                   |    | Other Pointer  | ---> | "commit", 0x0 |
    // |-------------------|    |----------------|      └---------------┘
    // | Null              |    | 0x0            |
    // |-------------------|    |----------------|       ┌-----------------------------┐
    // | Env Pointers...   |    | Pointer,       | ----> | "HOME=/home/ghostbird", 0x0 |
    // |                   |    | Other Pointer  | ---┐  └-----------------------------┘
    // |-------------------|    |----------------|    |
    // | Null              |    | 0x0            |    |   ┌---------------------------┐
    // |-------------------|    |----------------|    └-> | "PATH=/bin:/usr/bin", 0x0 |
    // | Auxv Type...      |    | AT_RANDOM      |        └---------------------------┘
    // | Auxv Value...     |    | Union->Pointer | -┐
    // |-------------------|    |----------------|  |   ┌---------------------------┐
    // | AT_NULL Auxv Pair |    | AT_NULL (0x0)  |  └-> | [16-bytes of random data] |
    // └-------------------┘    | Undefined      |      └---------------------------┘
    //                          └----------------┘

    // Check that `stack_pointer` is where (and what) we expect it to be.
    debug_assert_ne!(stack_pointer, null());
    debug_assert_eq!(stack_pointer.addr() & 0b1111, 0); // 16-byte aligned

    let arg_count = *stack_pointer;
    let arg_pointer = stack_pointer.add(1).cast::<*const u8>();

    debug_assert_eq!((*arg_pointer.add(arg_count)), null()); // args are null-terminated

    let env_pointer = arg_pointer.add(arg_count + 1);

    // Find the end of the environment variables + null-terminator + 1
    let auxv_pointer = (0..)
        .map(|i| env_pointer.add(i))
        .find(|&ptr| (*ptr).is_null())
        .unwrap_unchecked() // SAFETY: I mean, it's an infinite iterator. It'll segfault before it's None...
        .add(1)
        .cast::<AuxiliaryVectorItem>();

    todo!();
}
```

> <b style="color: var(--love);">SAFETY:</b> It's generally a terrible idea to use `unwrap_unchecked`, but this is an infinite iterator. The value will never be `None` and `unwrap` segfaults at this point anyway, actually the whole panic system does, so...

<br/>

This code won't compile until we define `AuxiliaryVectorItem`, and as mentioned above ^, a `debug_assert` will segfault at this point, but it's good enough for 11:30 at night.

<br/>

With a `AuxiliaryVectorItem` definition, we'll be able to identify other details like the program headers location `AT_PHDR`, the entry point `AT_ENTRY`, page size `AT_PAGE_SIZE`, and the base address of the `Elf` interpreter (dynamic linker/loader) `AT_BASE`.

<br/>

> I'm going to bed now, have a nice night! I'll start writing the next chapter tomorrow, but I have no clue when I'll finish. 🐸 <!-- -> [Next: Chapter 3]() -->

