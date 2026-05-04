# An Emulator for Some Imaginary Silicon 🫧

Before we can make our compiler (for a CPU that doesn't exist) we need a way to quickly iterate and test it. For that I'll write an emulator, both to be called from our unit tests programmatically, and eventually implemented in the CLI.

<br/>

The foundation of this chapter is boring (🏗️🕳️) — the emulator is super easy to write. But my usage of bitfields here influenced every project after this, and I've included a little taste (🍱) of the bitwise shenanigans that will be the focus of the next chapter.

<br/>

Chapters:
1. [Language Specification 🧬🏗️](/projects/dianac/diana_compiled_language_specification)
2. [An Emulator for Some Imaginary Silicon 🫧](/projects/dianac/an_emulator_for_some_imaginary_silicon)(you are here)
<!-- 3. [Bitwise Logic & My Compiler ⚙️🧮⚡](/projects/dianac/bitwise_logic_and_my_compiler) -->

<br/>
<details>
<summary><b>Table of Contents:</b></summary>

- [An Emulator for Some Imaginary Silicon 🫧](#an-emulator-for-some-imaginary-silicon)
  - [Machine State](#machine-state)
    - [Bit Field Magic ✨](#bit-field-magic)
      - [Instruction](#instruction)
      - [Operation](#operation)
      - [Operand](#operand)
      - [Special Instructions](#special-instructions)
    - [Program Counter 🧮](#program-counter)
    - [Memory 🏪](#memory)
      - [The Dum(b)? Lookup Table That Shouldn't Exist](#the-dumb-lookup-table-that-shouldnt-exist)
  - [Execution ⚙️](#execution)
  - [The Machine Runs ⚡](#the-machine-runs)

</details>

## Machine State


### Bit Field Magic ✨

My ISA uses 6-bit bytes and 12-bit words, neither of which are natively supported in Rust. Thankfully [Dan Lehmann](https://github.com/danlehmann) wrote a crate called [arbitrary-int](https://crates.io/crates/arbitrary-int) — it gives us types like `u6` and `u12` backed by the next-largest native integer, with arithmetic that wraps at the correct width.

<br/>

To parse our instructions we need to decode individual bits of our `u6` words. I hate using bitwise or (`|`) + bitwise and (`&`) to isolate the target fields. I instead use [bitbybit](https://crates.io/crates/bitbybit), written by the same author as arbitrary-int.

<br/>

> <b style="color: var(--foam);">NOTE:</b> This was the first time I ever used these two crates, but since then I use them everywhere. From serial protocol implementations, to the structures of the [Miros](https://github.com/5-pebbles/miros) C standard library.

#### Instruction

The `#[bitfield(u6)]` macro wraps a `u6` and generates typed accessors for each bit range. This maps directly to the ISA's `[XX][YY][ZZ]` layout from the [spec](/projects/dianac/diana_compiled_language_specification#instructions). Bits 4–5 are the operation, 2–3 and 0–1 are the two operands:

```rs
use arbitrary_int::u6;
use bitbybit::{bitenum, bitfield};

#[bitfield(u6)]
pub struct Instruction {
    #[bits(4..=5, rw)]
    pub operation: Operation,
    #[bits(2..=3, rw)]
    pub one: Register,
    #[bits(0..=1, rw)]
    pub two: Register,
}
```

Calling `instruction.operation()` extracts bits 4–5 and returns an `Operation` without any manual masking.

#### Operation

All four 2-bit values are assigned a variant, so `exhaustive = true` lets the generated constructor return the enum directly instead of a `Result`:

```rs
#[derive(Debug, PartialEq)]
#[bitenum(u2, exhaustive = true)]
pub enum Operation {
    Nor = 0b00,
    Pc = 0b01,
    Load = 0b10,
    Store = 0b11,
}
```

#### Operand

Same pattern for the operand identifier. `Immediate` allows variable length instructions, when the CPU encounters it, instead of reading a register it advances the program counter and uses the next word as a literal value:

```rs
#[derive(Debug, PartialEq)]
#[bitenum(u2, exhaustive = true)]
pub enum Register {
    A = 0b00,
    B = 0b01,
    C = 0b10,
    Immediate = 0b11,
}
```

`NOR A B` is one word, but `NOR A 0x1F` is two.

```
NOR A B
┌─────────────────┐
│ 0 0 │ 0 0 │ 0 1 │
│ NOR │  A  │  B  │
└─────────────────┘

NOR A 0x1F
┌─────────────────╥─────────────┐
│ 0 0 │ 0 0 │ 1 1 ║ 0 1 1 1 1 1 │
│ NOR │  A  │ IMM ║    0x1F     │
└─────────────────╨─────────────┘
```

The immediate value trails the instruction in memory.

#### Special Instructions

NOR's first operand is the destination, `NOR A B` stores the result in `A`. You can't NOR into an immediate value; the bit pattern `00_11_XX` (`NOR Immediate ...`) is nonsensical. That's four encodings, `001100` through `001111`, that can never appear as valid instructions.

<br/>

I didn't have a use for them until I started writing the compiler. The language has a conditional jump keyword called `LIH` (Logic Is Hard), it is the last keyword I implemented and is by far the most complicated, slow, and expensive (🪙).

<br/>

LIH works by evaluating a condition into either `0` or `3`, then adding that to the program counter's low half to skip past a jump instruction that would've followed the taken path (said jump instruction is of course three words long). The program counter is 12 bits, but it's stored as two 6-bit words — a high half and a low half. Adding `3` to the low half is trivial-ish unless you're near the top of its range. If the low half is `62` and you add `3`, you get `65` — which overflows a 6-bit value. Suddenly you need carry propagation into the high half, which means you need a spare register to hold intermediate state. With only three registers, all of which are already clobbered by the condition evaluation, there's nowhere to put the carry without reserving memory addresses as scratch space for register spills.

<br/>

The compiler tracks `next_address` as it emits instructions — it knows the runtime address of every word at compile time. So instead of solving 6-bit carry arithmetic with no free registers, I just pad with NOPs:

```rs
// Adding a six bit tuple is complicated and expensive.
// Solution: skip the 6-bit overflow.
while self.next_address & u12::new(0b111111) > u12::new(0b111111 - 3) {
    self.nop();
}
```

If the label would land within 3 of the boundary, keep emitting NOPs until it doesn't. The addition never overflows because the compiler guarantees the low half is always low enough. Problem circumvented, it is both simpler and faster (🏎️).

<br/>

But I didn't have a NOP instruction — which brought me back to those four dead encodings. `001100` became NOP, `001111` became HLT, and the two in between remain reserved for future instructions with no operands:

```
Special Instructions (NOR Immediate ...)
┌─────────────────┐
│ 0 0 │ 1 1 │ 0 0 │  NOP  (001100)
│ 0 0 │ 1 1 │ 0 1 │  ---  (001101)  reserved
│ 0 0 │ 1 1 │ 1 0 │  ---  (001110)  reserved
│ 0 0 │ 1 1 │ 1 1 │  HLT  (001111)
└─────────────────┘
```

### Program Counter 🧮

The program counter is a 12-bit address — same `#[bitfield]` pattern as `Instruction`, this time backed by a `u12` with named `high` and `low` halves. It needs to be shared, both CPU and memory subsystem need access for execution tracking, and to expose it as ROM at `0xF3E`–`0xF3F`. I could've wrapped it in `Rc<Mutex<T>>` where `Clone` creates a second handle to the same data, but decided that just passing it to memory `read` calls was simpler / more correct:

```rs
use arbitrary_int::{u12, u6};
use bitbybit::bitfield;

#[bitfield(u12)]
#[derive(Debug, Default)]
pub struct ProgramCounter {
    #[bits(6..=11, rw)]
    high: u6,
    #[bits(0..=5, rw)]
    low: u6,
}

impl ProgramCounter {
    pub fn set(&mut self, value: (u6, u6)) {
        *self = Self::default().with_high(value.0).with_low(value.1);
    }

    pub fn increment(&mut self) {
        *self = Self::new_with_raw_value(self.raw_value().wrapping_add(u12::new(1)));
    }

    pub fn as_tuple(&self) -> (u6, u6) {
        (self.high(), self.low())
    }
}
```

Because the bitfield is backed by a single `u12`, increment is just a wrapping add on the raw value, the full address space rolls from `0xFFF` to `0x000`.

### Memory 🏪

3902 bytes of RAM, a read-only window into the program counter, and two rotate lookup tables:

```rs
use arbitrary_int::u6;

use crate::{emulation::program_counter::ProgramCounter, utils::tuple_as_usize};

const RAM_SIZE: usize = 3902;

pub struct Memory {
    pub ram: [u6; RAM_SIZE],
}

impl Memory {
    pub fn new() -> Self {
        Self {
            ram: [u6::default(); RAM_SIZE],
        }
    }

    pub fn store_array(&mut self, offset: usize, machine_code: &[u6]) {
        self.ram[offset..offset + machine_code.len()].copy_from_slice(&machine_code);
    }

    pub fn read(&self, address: (u6, u6), pc: ProgramCounter) -> u6 {
        let as_usize = tuple_as_usize(address);

        match as_usize {
            0x000..=0xF3D => self.ram[as_usize],
            0xF3E => pc.low(),
            0xF3F => pc.high(),
            0xF80..=0xFBF => address.1.rotate_left(1),
            0xFC0..=0xFFF => address.1.rotate_right(1),
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, address: (u6, u6), value: u6) {
        let as_usize = tuple_as_usize(address);

        match as_usize {
            0x000..=0xF3D => self.ram[as_usize] = value,
            // TODO I don't know what to use this for, but I am not letting this many addresses go to waste
            0xF3E..=0xFFF => todo!(),
            _ => unreachable!(),
        }
    }
}
```

NOR is a [universal logic gate](https://en.wikipedia.org/wiki/NOR_logic) — any boolean function can be built from NOR alone. Combined with stateful registers, you can compute any operation on an individual bit: AND, OR, XOR, NOT, all decomposed into sequences of NOR. But NOR is bitwise. Bit 5 of the output depends on bit 5 of the inputs. Bit 0 depends on bit 0. Each bit is trapped in its own slot / column with no way to influence its siblings. You're unable to do addition, multiplication, or any other operation requiring bit permutation.

#### The Dum(b)? Lookup Table That Shouldn't Exist

How do you physically move bits between positions so they can interact? Writing this two years later, my obvious answer is: the two reserved [special instruction](#special-instructions) encodings (`001101`, `001110`) could be mapped to rotate C left and rotate C right respectively. But nope, I of course implemented <u>a whole fucking lookup table</u>, welp (🫠)... it's another thing to fix in the Diana III ISA.

<br/>

I guess I thought the instruction encoding is full? So instead of fighting for the most valuable real estate in the ISA (instruction layout), I used the cheapest (address space). When you load from `0xF80 + x`, you get `x` rotated left by one bit. The low 6 bits of the address are both the input and the table index; the read result is the rotation of that index. The table has limited variance, it's a fixed permutation from 64 inputs to 64 outputs.

<br/>

Again - all dumb, didn't need to happen.


## Execution ⚙️

The CPU state lives in a struct, three registers, a program counter, and memory:

```rs
pub struct InteractiveState {
    pub a: u6,
    pub b: u6,
    pub c: u6,
    pub memory: Memory,
    pub program_counter: ProgramCounter,
}
```

Before we can execute an instruction we need to resolve its operands. Registers are straightforward, but `Immediate` advances the program counter to consume the trailing word:

```rs
fn consume_operand(&mut self, operand: Register) -> u6 {
    match operand {
        Register::A => self.a,
        Register::B => self.b,
        Register::C => self.c,
        Register::Immediate => {
            self.program_counter.increment();
            self.memory
                .read(self.program_counter.as_tuple(), self.program_counter)
        }
    }
}
```

An instruction with two immediate operands increments the PC twice during operand consumption, then once more after — advancing past all three words.

<br/>

The full execution cycle is as follows: read raw word, check for special instruction, decode, consume operands, advance PC, execute:

```rs
pub fn consume_instruction(&mut self) {
    let raw_value = self
        .memory
        .read(self.program_counter.as_tuple(), self.program_counter);

    // TODO add special instructions
    match raw_value.value() {
        0b001100 /* Nop */ => {
            self.program_counter.increment();
            return;
        }
        0b001101 => unimplemented!(),
        0b001110 => unimplemented!(),
        0b001111 /* Hlt */ => {
            self.program_counter.increment();
            return;
        },
        _ => (),
    }

    let instruction = Instruction::new_with_raw_value(raw_value);

    let operand_one = self.consume_operand(instruction.one());
    let operand_two = self.consume_operand(instruction.two());
    self.program_counter.increment();

    match instruction.operation() {
        Operation::Nor => {
            let new_value = !(operand_one | operand_two);
            match instruction.one() {
                Register::A => self.a = new_value,
                Register::B => self.b = new_value,
                Register::C => self.c = new_value,
                _ => unreachable!(),
            }
        }
        Operation::Pc => self.program_counter.set((operand_one, operand_two)),
        Operation::Load => {
            self.c = self
                .memory
                .read((operand_one, operand_two), self.program_counter);
        }
        Operation::Store => self.memory.write((operand_one, operand_two), self.c),
    }
}
```

The two reserved patterns (`0b001101`, `0b001110`) hit `unimplemented!()` for now; NOP and HLT are caught the same way before decoding.

<br/>

HLT doesn't set an internal flag — `consume_instruction` treats it identically to NOP. Halting is the caller's responsibility: `is_halt()` peeks at the word under the program counter and the caller decides whether to stop:

```rs
pub fn is_halt(&self) -> bool {
    self.memory
        .read(self.program_counter.as_tuple(), self.program_counter)
        == u6::new(0b001111)
}

pub fn consume_until_halt(&mut self) {
    while !self.is_halt() {
        self.consume_instruction();
    }
}
```


## The Machine Runs ⚡

The compiler doesn't exist yet, but we can hand-assemble a program and run it in a unit test directly. For quick reference, NOR is just `!(A | B)`:

| A | B | -> | NOR |
|:-:|:-:|----|:---:|
| 1 | 1 |    |  0  |
| 1 | 0 |    |  0  |
| 0 | 1 |    |  0  |
| 0 | 0 |    |  1  |

`NOR A 0b111111` zeros A — OR with all-ones always saturates, NOR flips it to zero:

```txt
A         1 0 1 0 1 0  (garbage)
IMM       1 1 1 1 1 1
──────────────────────
A | IMM   1 1 1 1 1 1
!         0 0 0 0 0 0  ← A zeroed
```

From there, `NOR A x` with A zeroed gives the inverse of `x`:

```txt
A         0 0 0 0 0 0
IMM       0 1 0 1 0 1
──────────────────────
A | IMM   0 1 0 1 0 1
!         1 0 1 0 1 0  ← A = !IMM
```

To invert A back, `NOR A A` — NOR of a value with itself is NOT:

```txt
A         1 0 1 0 1 0
A         1 0 1 0 1 0
──────────────────────
A | A     1 0 1 0 1 0
!         0 1 0 1 0 1  ← A inverted
```

Zero, load inverse, invert. The only difference between this and the `MOV` instruction we'll eventually implement is that `MOV` flips any constants at compile time.

<br/>

In six words:

```
Address  Binary    Meaning
──────────────────────────────────────
0x000    000011    NOR A Immediate
0x001    111111    0b111111
0x002    000011    NOR A Immediate
0x003    010101    0b010101 (the immediate value)
0x004    000000    NOR A A
0x005    001111    HLT
```

And as a unit test:

```rs
#[test]
fn hand_assembled_nor() {
    let mut state = InteractiveState::new();
    state.a = u6::new(0b101010); // garbage

    state.memory.store_array(0, &[
        u6::new(0b000011), // NOR A Immediate
        u6::new(0b111111), //   0b111111 (zero A)
        u6::new(0b000011), // NOR A Immediate
        u6::new(0b010101), //   0b010101 (load complement)
        u6::new(0b000000), // NOR A A   (NOT)
        u6::new(0b001111), // HLT
    ]);

    state.consume_until_halt();
    assert_eq!(state.a, u6::new(0b010101));
}
```

That's it. Now I need a compiler, so next time this is its problem, not mine. ✨

<!-- [Next: Chapter 3](/projects/dianac/bitwise_logic_and_my_compiler) -->
