<head>
  <title>Slayer of Dragons, Eater of Bugs | Auxv.org</title>
  <meta name="author" content="Owen Friedman">
</head>

# Slayer of Dragons, Eater of Bugs 🐔

If you've been following along with the chapters thus far, you should know we've been walking through a small minefield. Miros is a from-scratch runtime — dynamic linker, libc, pthreads — written in Rust. I've been trying to make minimal concession in my code style and architecture, but there are things we can't avoid: e.g., we don't have a working allocator.

<br/>

Error handling doesn't work yet either, so surprise limitations show up as segfaults. I've mentioned the issues, but never walked through how to diagnose one encountered in the wild. That will be the subject of this chapter, be prepared to get out your favorite debugger.

<br/>

Here is the story of how a 160-byte struct, a compiler builtin, and the Global Offset Table conspired to ruin my afternoon. 🏜️

<br/>
<details>
<summary><b>Table of Contents:</b></summary>

- [Slayer of Dragons, Eater of Bugs 🐔](#slayer-of-dragons-eater-of-bugs)
  - [An Invisible Builtin](#an-invisible-builtin)
    - [Exploring the Assembly ⚙️🏔️](#exploring-the-assembly)
    - [Unmasking the Phantom Call 👻](#unmasking-the-phantom-call)
    - [The Chicken and the Egg 🐔🥚](#the-chicken-and-the-egg)
    - [A Quick Fix 🚑🩹](#a-quick-fix)
  - [Dynamic Dispatch & VTables](#dynamic-dispatch--vtables)
    - [Trait Objects Under the Hood 🧬](#trait-objects-under-the-hood)
    - [Same Egg, Different Chicken 🐣](#same-egg-different-chicken)
    - [Static Before Dynamic 🪺](#static-before-dynamic)
  - [Lessons Learned 📝](#lessons-learned)

> <b style="color: var(--foam);">Note:</b> The chapter's title comes from my battle cry "I am the Slugcat, Slayer of Dragons, Eater of Bugs" while playing [Rain World](https://en.wikipedia.org/wiki/Rain_World). I refused to walk away from a fight; this is the main reason I've never "completed" the game. I died a lot... 💀

</details>


## An Invisible Builtin

The code we wrote last chapter has a couple bugs, this was <u>totally intentional</u> on my part. It constructs an `ObjectData<NonDynamic>`, wraps a few stratagems into a pipeline, then runs them. All valid Rust:

```rs
// Relocate ourselves and initialize thread local storage:
let miros = if auxv_info.base.is_null() {
    ObjectData::<NonDynamic>::from_program_headers(&program_header_table)
} else {
    ObjectData::from_base(auxv_info.base)
};

let relocate = Relocate::new();
let thread_local_storage =
    ThreadLocalStorage::new(auxv_info.pseudorandom_bytes.as_ref().unwrap_unchecked());
let init_array = InitArray::new(arg_count, arg_pointer, env_pointer, auxv_pointer);

let stratagems: &[&dyn Stratagem<ObjectDataSingle>] =
    &[&relocate, &thread_local_storage, &init_array];

let pipeline = ObjectPipeline::new(stratagems);
let _ = pipeline.run_pipeline(miros);
```

`run_pipeline` takes `miros` **by value**:

```rs
pub fn run_pipeline(&self, mut object_data: T) -> Result<(), MirosError> {
    self.pipeline
        .into_iter()
        .try_for_each(|stratagem| stratagem.run(&mut object_data))
}
```

Running our test binary:

```sh
❯ ./examples/print_deadbeef
# Job 1, './examples/print_deadbeef' terminated by signal SIGSEGV (Address boundary error)
```


### Exploring the Assembly ⚙️🏔️

My favorite debugger is `rust-lldb` (🪱), which ships with the Rust toolchain. The usual workflow is as follows: run, let it crash, collect the backtrace, set a breakpoint near the fault, disassemble, and single-step to the faulting instruction.

The segfault happens on line 105 — the `run_pipeline` call. But not *inside* `run_pipeline`.

```
Process 378540 stopped
* thread #1, name = 'print_deadbeef', stop reason = step over
    frame #0: 0x00007ffff7fac87f miros`miros::start::relocate_and_calculate_jump_address::h81e07157b6e2e558(stack_pointer=0x00007fffffffe7b0) at mod.rs:105:35
   102 	       &[&relocate, &thread_local_storage, &init_array];
   103 	
   104 	   let pipeline = ObjectPipeline::new(stratagems);
-> 105 	   let _ = pipeline.run_pipeline(miros);
   106 	
   107 	   println!("test");
   108
```

After a quick disassembly we see the segfault is right before it, at a `call rax` instruction:

```x86asm
    0x7ffff7fac85e <+2014>: lea    rdi, [rsp + 0x3d0]
    0x7ffff7fac866 <+2022>: mov    qword ptr [rsp + 0x8], rdi
    0x7ffff7fac86b <+2027>: lea    rsi, [rsp + 0x2c0]
    0x7ffff7fac873 <+2035>: mov    edx, 0xa0
    0x7ffff7fac878 <+2040>: mov    rax, qword ptr [rip + 0x4e9b1]
->  0x7ffff7fac87f <+2047>: call   rax ; signal SIGSEGV
    0x7ffff7fac881 <+2049>: mov    rsi, qword ptr [rsp + 0x8]
    0x7ffff7fac886 <+2054>: lea    rdi, [rsp + 0x3c0]
    0x7ffff7fac88e <+2062>: call   0x7ffff7fae850 ; miros::objects::object_pipeline::ObjectPipeline$LT$T$GT$::run_pipeline::h9c16a6e84cf4ddc1 at object_pipeline.rs:14
```

Look at the setup: `rdi = dest`, `rsi = src`, `edx = 0xa0` (160 bytes). This is the [System V AMD64 calling convention](https://refspecs.linuxbase.org/elf/x86_64-abi-0.99.pdf) for a three-argument function. Then it loads a function pointer from `[rip + 0x4e9b1]` and calls it.

<br/>

We can find the base address at which our program was loaded via `image list miros` in lldb:

```
[  0] D4B5D5D2-633C-8E62-5035-B02C26B98237-1B344A43 0x00007ffff7f8e000 /home/ghostbird/git/miros/target/debug/miros 
```

This means our base address is `0x00007ffff7f8e000`.

<br/>

The function's address is loaded from `[rip + 0x4e9b1]`. RIP-relative addressing resolves against the next instruction (`0x7ffff7fac87f`), so the GOT entry lives at `0x7ffff7fac87f + 0x4e9b1`.


### Unmasking the Phantom Call 👻

We know something is being called through the GOT, but we don't know *what*. To find out, we need to work backwards from the runtime address to a symbol name. Subtracting the base address gives us the relocation's file offset:

```
relocation_address = ((0x7ffff7fac87f + 0x4e9b1) - 0x7ffff7f8e000) = 0x6d230
```

Now we can look up what lives at that offset in the relocation table:

```sh
❯ readelf -r ./target/debug/miros | grep 6d230
00000006d230  000000000008 R_X86_64_RELATIVE   22a50
```

It's an `R_X86_64_RELATIVE` relocation. For this type, the `r_addend` field (`22a50`) is the symbol's file offset — no symbol table lookup needed by the linker, it just adds the base address. But *we* can still use it to identify the function:

```sh
❯ readelf -s ./target/debug/miros | grep 22a50
    39: 0000000000022a50    27 FUNC    GLOBAL DEFAULT   14 memcpy
  1267: 0000000000022a50    27 FUNC    GLOBAL DEFAULT   14 memcpy
```

It's `memcpy`. The compiler silently emitted a `memcpy` call to copy our 160-byte `ObjectData` struct into `run_pipeline`'s stack frame — because we passed it by value. And because this is position-independent code, it routed the call through the **Global Offset Table**, which hasn't been relocated yet.


### The Chicken and the Egg 🐔🥚

This is a fundamental bootstrapping problem. Dynamic linkers have to relocate themselves before they can do anything useful. But the Rust compiler silently inserts `memcpy` calls whenever it needs to copy a struct that's too large to fit in registers (~40 bytes on x86_64, though the exact threshold depends on the target and optimization level). These calls go through the GOT, which requires relocations to have been applied first.

<br/>

I've actually [run into this before](https://users.rust-lang.org/t/how-to-force-inlining-or-avoid-calls-to-the-plt/123173) during a previous refactor. Last time it was `IRELATIVE` relocations and glibc's `__new_memcpy_ifunc` selector. This time it's `R_X86_64_RELATIVE` and our own `memcpy`. The root cause is the same: LLVM always emits `memcpy` through the GOT for PIC/PIE code, regardless of whether the symbol is defined in the same binary.


### A Quick Fix 🚑🩹

The `memcpy` was being emitted because `run_pipeline` took `object_data` by value. Passing a 160-byte struct by value requires a copy. Changing it to take a mutable reference passes an 8-byte pointer instead — no memcpy needed.

From:

```rs
pub fn run_pipeline(&self, mut object_data: T) -> Result<(), MirosError> {
```

Into:

```rs
pub fn run_pipeline(&self, object_data: &mut T) -> Result<(), MirosError> {
```

No copy, no GOT lookup, no crash, totally genius (🧠🛸).


## Dynamic Dispatch & VTables

With the `memcpy` crash behind us, we run the binary again:

```sh
❯ ./examples/print_deadbeef
# Job 1, './examples/print_deadbeef' terminated by signal SIGSEGV (Address boundary error)
```

Different crash, same symptom (💀). The backtrace this time:


```
* thread #1, name = 'print_deadbeef', stop reason = signal SIGSEGV: address not mapped to object (fault address=0x0)
  * frame #0: 0x0000000000000000
    frame #1: 0x00007ffff7fae86e miros`miros::objects::object_pipeline::ObjectPipeline$LT$T$GT$::run_pipeline::_$u7b$$u7b$closure$u7d$$u7d$::h11b4bc2099db2663(stratagem=0x00007fffffffe5a0) at object_pipeline.rs:17:49
    frame #2: 0x00007ffff7faa8e2 miros`core::iter::traits::iterator::Iterator::try_for_each::call::_$u7b$$u7b$closure$u7d$$u7d$::h9697ba5c11708478((null)=<unavailable>, x=0x00007fffffffe5a0) at iterator.rs:2485:26
    frame #3: 0x00007ffff7fadcab miros`core::iter::traits::iterator::Iterator::try_fold::he33431230abc791f(self=0x00007fffffffe1f8, init=<unavailable>, f={closure_env#0}<&&dyn miros::objects::strategies::Stratagem<miros::objects::object_data::ObjectData<miros::objects::object_data::NonDynamic>>, core::result::Result<(), miros::error::MirosError>, miros::objects::object_pipeline::{impl#0}::run_pipeline::{closure_env#0}<miros::objects::object_data::ObjectData<miros::objects::object_data::NonDynamic>>> @ 0x00007fffffffe170) at iterator.rs:2427:21
    frame #4: 0x00007ffff7fad7a3 miros`core::iter::traits::iterator::Iterator::try_for_each::h4d438c86fa6ff5c9(self=0x00007fffffffe1f8, f={closure_env#0}<miros::objects::object_data::ObjectData<miros::objects::object_data::NonDynamic>> @ 0x00007fffffffe1e0) at iterator.rs:2488:14
    frame #5: 0x00007ffff7fae849 miros`miros::objects::object_pipeline::ObjectPipeline$LT$T$GT$::run_pipeline::ha735bcc569c55452(self=0x00007fffffffe5d0, object_data=0x00007fffffffe4d0) at object_pipeline.rs:17:14
    frame #6: 0x00007ffff7fac845 miros`miros::start::relocate_and_calculate_jump_address::h81e07157b6e2e558(stack_pointer=0x00007fffffffe7b0) at mod.rs:105:22
    frame #7: 0x00007ffff7fac048 miros`_start + 12
```

We're jumping to address `0x0`, which is problematic for obvious reasons. And it's coming from *inside* `run_pipeline`, at the `stratagem.run(object_data)` call on line 17.

```
Process 456743 stopped
* thread #1, name = 'print_deadbeef', stop reason = breakpoint 1.1
    frame #0: 0x00007ffff7fae86b miros`miros::objects::object_pipeline::ObjectPipeline$LT$T$GT$::run_pipeline::_$u7b$$u7b$closure$u7d$$u7d$::h11b4bc2099db2663(stratagem=0x00007fffffffe5a0) at object_pipeline.rs:17:49
   14  	   pub fn run_pipeline(&self, object_data: &mut T) -> Result<(), MirosError> {
   15  	       self.pipeline
   16  	           .into_iter()
-> 17  	           .try_for_each(|stratagem| stratagem.run(object_data))
   18  	   }
   19  	}
```


### Trait Objects Under the Hood 🧬

[Ok](https://en.wikipedia.org/wiki/OK#Boston_abbreviation_fad), I know this is a non-relocated vtable because the source code is calling via dynamic dispatch. To understand why this crashes, we need to understand how `&dyn Stratagem<T>` works. In Rust, a reference to a `dyn` trait object is a **fat pointer** — two addresses instead of one:

```
&dyn Stratagem<T>:
┌──────────────────┬──────────────────┐
│    data_ptr      │   vtable_ptr     │
│    (8 bytes)     │   (8 bytes)      │
└──────────────────┴──────────────────┘
```

The first pointer is a reference to a concrete struct (e.g. `&Relocate`). And the second points to a compiler-generated **vtable** — a static table of function pointers for that trait implementation:

```
vtable for <Relocate as Stratagem<ObjectDataSingle>>:
┌──────────────────┐
│  drop_in_place   │  offset 0x00
├──────────────────┤
│  size            │  offset 0x08
├──────────────────┤
│  align           │  offset 0x10
├──────────────────┤
│  Stratagem::run  │  offset 0x18
└──────────────────┘
```



The first three fields are always present in every vtable, but their values differ per type. `drop_in_place` is the destructor, and `size` and `align` help us interpret the data pointer. Our trait only has one method, if we were to add another it would be defined with the next available offset, each one getting an entry in the vtable in the order in which they were defined.

<br/>

Instead of the compiler emitting a function address, it saves both the vtable address and method offset. There's not much identifying information for this at runtime — kinda untrue, vtables show up in [DWARF](https://en.wikipedia.org/wiki/DWARF), but that's debug info only. Anyway you can still see what's happening in the assembly:

```x86asm
    0x7ffff7fae850 <+0>:  sub    rsp, 0x18
    0x7ffff7fae854 <+4>:  mov    rcx, rdi
    0x7ffff7fae857 <+7>:  mov    qword ptr [rsp + 0x8], rcx
    0x7ffff7fae85c <+12>: mov    qword ptr [rsp + 0x10], rsi
    0x7ffff7fae861 <+17>: mov    rdi, qword ptr [rsi]         ; rdi = data_ptr (self for the stratagem)
    0x7ffff7fae864 <+20>: mov    rax, qword ptr [rsi + 0x8]   ; rax = vtable_ptr
    0x7ffff7fae868 <+24>: mov    rsi, qword ptr [rcx]         ; rsi = object_data (&mut T)
->  0x7ffff7fae86b <+27>: call   qword ptr [rax + 0x18]       ; call vtable[3] = run() — THIS JUMPS TO 0x0
    0x7ffff7fae86e <+30>: add    rsp, 0x18
    0x7ffff7fae872 <+34>: ret
```

The crashing instruction is `call qword ptr [rax + 0x18]` — an indirect call through the vtable at slot `+0x18` (the `run` method, 4th pointer in the Rust vtable).

<br/>

Load the vtable pointer, index into it, call the function. Simple enough — except just like a function pointer in the Global Offset Table, the runtime address can't be known at compile time, vtable pointers require relocations.


### Same Egg, Different Chicken 🐣

From the assembly, we know that the crash is at `call qword ptr [rax + 0x18]` — a vtable dispatch where `rax` holds the vtable pointer. We can read `rax` with the `rust-lldb` command:

```
(lldb) reg r rax
     rax = 0x00007ffff7ff7ba8
```

Using the base address from earlier (`0x00007ffff7f8e000`). The faulting instruction indexes into `rax` at offset `+0x18`, so the run method entry it's trying to call lives at:

```
run_method_vtable_slot = (0x00007ffff7ff7ba8 + 0x18) - 0x00007ffff7f8e000 = 0x69bc0
```

> <b style="color: var(--foam);">Note:</b> The base address may change between executions, make sure to check each time you start the debugger. 🦋❌

<br/>

```sh
❯ readelf -r ./target/debug/miros | grep 69bc0
000000069bc0  000000000008 R_X86_64_RELATIVE   27ab0
```

Before relocations are applied, that address is unresolved. The chicken-and-egg problem isn't a one-time obstacle — it's the recurring antagonist of this project.


### Static Before Dynamic 🪺

The pipeline is mostly aesthetic anyway, so we can just run relocate via static dispatch and then execute the remainder of the pipeline via `dyn` dispatch:

```rs
// Relocate ourselves and initialize thread local storage:
let mut miros = if auxv_info.base.is_null() {
    ObjectData::<NonDynamic>::from_program_headers(&program_header_table)
} else {
    ObjectData::from_base(auxv_info.base)
};

let relocate = Relocate::new();
let thread_local_storage =
    ThreadLocalStorage::new(auxv_info.pseudorandom_bytes.as_ref().unwrap_unchecked());
let init_array = InitArray::new(arg_count, arg_pointer, env_pointer, auxv_pointer);

let stratagems: &[&dyn Stratagem<ObjectDataSingle>] = &[&thread_local_storage, &init_array];

let pipeline = ObjectPipeline::new(stratagems);
let _ = relocate
    .run(&mut miros)
    .and_then(|_| pipeline.run_pipeline(&mut miros));
```

With `Relocate` called via static dispatch, the GOT and vtables are patched before anything tries to use them. `ThreadLocalStorage` and `InitArray` can then safely go through the pipeline — their vtable pointers will resolve correctly because the relocations have already been applied.


## Lessons Learned 📝

You *can* write high-level Rust at this level — generics, trait objects, iterators — as long as you understand what the compiler will do with it. There are no formal guarantees about codegen, but the compiler isn't adversarial. It *could* technically emit a `memcpy` at any time (and you should be careful), but it has no reason to copy an object passed by reference, so it won't. (usually)

<br/>

> I wrote this chapter out of order, as in most of the preceding writing doesn't exist yet. I wanted to document it while I had the debugger logs on hand. Hopefully it will find its place someday. o7
