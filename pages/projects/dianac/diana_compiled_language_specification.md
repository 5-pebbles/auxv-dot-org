# Diana Compiled Language Spec 🧬🏗️

I'd like to design my own CPU someday. I haven't gotten to it yet, but while playing around I do occasionally write my own ISAs (Instruction Set Architectures) 📝. Then I roughly figure out how to implement them in hardware before deciding I don't feel like spending the next 6 months in KiCad and scrapping the idea...

<br/>

I've been doing this for years, when I was 19 I decided I'd like to learn how compiler internals work, and what better way to learn than implementing an abstraction layer over one of my esoteric ISA's. I'm writing these articles 2-years after the fact, so excuse any technical oversights.

<br/>

Chapters:
1. [Language Specification 🧬🏗️](/projects/dianac/diana_compiled_language_specification)(you are here)
2. [An Emulation REPL 🐚](/projects/dianac/an_emulation_repl)
3. [Bitwise Logic & My Compiler ⚙️🧮⚡](/projects/dianac/bitwise_logic_and_my_compiler)

Below you'll find the specification for the language I'd eventually write.

<br/>

<details>
<summary><b>Table of Contents:</b></summary>

- [Diana Compiled Language Spec 🧬🏗️](#diana-compiled-language-spec)
  - [The Diana-II ISA 🔩](#the-dianaii-isa)
    - [Instructions](#instructions)
    - [Operands](#operands)
    - [Memory Layout 🗺️](#memory-layout)
  - [Lexical Conventions](#lexical-conventions)
    - [Statements](#statements)
    - [Comments](#comments)
    - [Labels](#labels)
    - [Tokens](#tokens)
      - [Identifiers](#identifiers)
      - [Keywords](#keywords)
      - [Registers](#registers)
      - [Numerical Constants](#numerical-constants)
      - [Character Constants](#character-constants)
      - [Operators](#operators)
  - [Keywords, Operands, and Addressing](#keywords-operands-and-addressing)
    - [Operand Types](#operand-types)
    - [Addressing](#addressing)
  - [Side Effects ⚠️](#side-effects)
  - [Keyword Tables 📋](#keyword-tables)
    - [Bitwise Logic Keywords 🔣](#bitwise-logic-keywords)
    - [Shift and Rotate Keywords](#shift-and-rotate-keywords)
    - [Arithmetic Keywords](#arithmetic-keywords)
    - [Memory Keywords 🧠](#memory-keywords)
    - [Jump Keywords 🦘](#jump-keywords)
    - [Miscellaneous Keywords](#miscellaneous-keywords)

</details>

## The Diana-II ISA 🔩

The Diana II is a 6-bit minimal instruction set computer designed around using `NOR` as a universal logic gate. `NOR` doesn't allow bit permutations, so I used rotate lookup tables to perform those.

- **byte size:** 6-bits.

- **endianness:** little-endian.

- **address size:** 12-bits (two 6-bit operands, first is higher order).

- **unique instructions:** 6.


### Instructions

| Binary |      Instruction     |  Description  |
|--------|----------------------|---------------|
|   00   |  `NOR [val] [val]`   |  Performs a negated OR on the first operand. |
|   01   |  `PC [val] [val]`    |  Sets the program counter to the address `[val, val]`. |
|   10   |  `LOAD [val] [val]`  |  Loads data from the address `[val, val]` into `C`. |
|   11   |  `STORE [val] [val]` |  Stores the value in `C` at the address `[val, val]`. |

**Layout:**

Each instruction is 6 bits in the format `[XX][YY][ZZ]`:

- **X:** 2-bit instruction identifier.
- **Y:** 2-bit first operand identifier.
- **Z:** 2-bit second operand identifier.

The first operand of NOR can't be immediate, so that allows another four instructions:

| Binary |   Instruction   | Description |
|--------|-----------------|-------------|
| 001100 | `NOP` | No operation; used for padding. |
| 001101 | `---` | Reserved for future use. |
| 001110 | `---` | Reserved for future use. |
| 001111 | `HLT` | Halts the CPU until the next interrupt. |


> <b style="color: var(--foam);">Note:</b> Instructions and operands are both uppercase because my 6-bit character encoding doesn't support lowercase...


### Operands

| Binary | Name | Description |
|--------|------|-------------|
| **00** |   A  | General purpose register. |
| **01** |   B  | General purpose register. |
| **10** |   C  | General purpose register. |
| **11** |   -  | Read next instruction as a value. |


### Memory Layout 🗺️

There are a total of 4096 unique addresses each containing 6 bits.

|     Address     |  Description  |
|-----------------|---------------|
| `0x000..=0xEFF` | General purpose RAM. |
| `0xF00..=0xF3D` | Reserved for future use. |
| `0xF3E..=0xF3F` | Program Counter(PC) (ROM). |
| `0xF40..=0xF7F` | Reserved for future use. |
| `0xF80..=0xFBF` | Left rotate lookup table (ROM). |
| `0xFC0..=0xFFF` | Right rotate lookup table (ROM). |


## Lexical Conventions

### Statements

A program consists of one or more files containing _statements_. A _statement_ consists of _tokens_ separated by whitespace and terminated by a newline character.

### Comments

A _comment_ can reside on its own line or be appended to a statement.  The comment consists of an octothorp (#) followed by the text of the comment and a terminating newline character.

### Labels

A _label_ can be placed before the beginning of a statement. During compilation the label is assigned the address of the following statement and can be used as a keyword operand.
A label consists of the `LAB` keyword followed by an _identifier_. Labels are global in scope and appear in the file's symbol table.

### Tokens

There are 6 classes of tokens:

- Identifiers
- Keywords
- Registers
- Numerical constants
- Character constants
- Operators

#### Identifiers

An identifier is an arbitrarily-long sequence of letters, underscores, and digits. The first character must be a letter or underscore. Uppercase and lowercase characters are equivalent.

#### Keywords

Keywords such as instruction mnemonics and directives are reserved and cannot be used as identifiers. For a list of keywords see the [Keyword Tables](#keyword-tables).

#### Registers

The Diana-II architecture provides three registers **\[A, B, C\]**; these are reserved and cannot be used as identifiers. Uppercase and lowercase characters are equivalent.

#### Numerical Constants

Numbers in the Diana-II architecture are unsigned 6-bit integers. These can be expressed in several bases:

- **Decimal.** Decimal integers consist of one or more decimal digits (0–9).
- **Binary.** Binary integers begin with “0b” or “0B” followed by one or more binary digits (0, 1).
- **Hexadecimal.** Hexadecimal integers begin with “0x” or “0X” followed by one or more hexadecimal digits (0–9, A–F). Hexadecimal digits can be either uppercase or lowercase.

#### Character Constants

A _character_ constant consists of a supported character enclosed in single quotes ('). A character will be converted to its numeric representation based on the table of supported characters below:

|    | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |  8  | 9 |   A   | B | C | D | E |  F  |
|:--:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:---:|:-:|:-----:|:-:|:-:|:-:|:-:|:---:|
| 0x | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |  8  | 9 |   =   | - | + | * | / |  ^  |
| 1x | A | B | C | D | E | F | G | H |  I  | J |   K   | L | M | N | O |  P  |
| 2x | Q | R | S | T | U | V | W | X |  Y  | Z | SPACE | . | , | ' | " |  \` |
| 3x | # | ! | & | ? | ; | : | $ | % |  \| | > |   <   | [ | ] | ( | ) |  \\ |

If a lowercase character is used, it will be converted to its uppercase representation.

#### Operators

The compiler supports the following operators for use in expressions. Operators have no assigned precedence. Expressions can be grouped in parentheses () to establish precedence.

|     |     |
|-----|-----|
|  !  | Logical NOT |
|  &  | Logical AND |
|  \| | Logical OR |
|  +  | Addition |
|  -  | Subtraction |
|  *  | Multiplication |
|  /  | Division |
|  >> | Rotate right |
|  << | Rotate left |

All operators except Logical NOT require two values and parentheses ():

- `(5 + 9 + 3)` = **17**
- `!0b111110` = **0b000001**
- `(2 + (2 * 5))` = **12**
- `(2 + 2 * 5)` = **20**


## Keywords, Operands, and Addressing

Keywords represent an instruction, set of instructions, or a directive. Operands are entities operated upon by the keyword. Addresses are the locations in memory of specified data.

### Operand Types

A keyword can have zero to three operands separated by whitespace characters. For instructions with a source and destination this language uses Intel's notation destination (left) then source (right).

There are 5 types of operands:

- **Immediate.** A 6-bit constant expression that evaluates to an inline value.
- **Register.** One of the three 6-bit general-purpose registers provided by the Diana-II architecture.
- **Either.** An immediate or a register operand.
- **Address.** A single 12-bit identifier or a pair of whitespace separated 6-bit either operands.
- **Conditional.** A pair of square brackets \[ \] containing a pair of 6-bit operands separated by whitespace and one of the following comparison operators:
    |      |      |
    |------|------|
    |  ==  | Equal |
    |  !=  | Not equal |
    |  >   | Greater |
    |  >=  | Greater or equal |
    |  <   | Less |
    |  <=  | Less or equal |

### Addressing

The Diana-II architecture uses 12-bit addressing. Labels can be split into two 6-bit immediate values by appending a colon followed by a 0 or 1, where `:0` is the high-order and `:1` is the low-order half. If a keyword requires an address it can be provided as two 6-bit values or a single 12-bit identifier:

- `LOD MAIN` = `LOD MAIN:0 MAIN:1`.


## Side Effects ⚠️

Any side effects will be listed in the notes of a keyword; read each carefully. If a keyword clobbers an unrelated register, it will select the first available in reverse alphabetical order, e.g.

- `NXOR C 0x27` will clobber **B**
- `NXOR A 0x27` will clobber **C**


## Keyword Tables 📋

Operands will be displayed in square brackets \[ \] using the following shorthand:

- `[reg]` = **register**
- `[imm]` = **immediate**
- `[eth]` = **either**
- `[add]` = **address**
- `[con]` = **conditional**

### Bitwise Logic Keywords 🔣

| Keyword | Description | Notes |
|---------|-------------|-------|
| `NOT [reg]` | bitwise logical NOT | - |
| `AND [reg] [eth]` | bitwise logical AND | The second register is flipped; its value can be restored with a `NOT` operation. If an immediate value is used, it is flipped at compile time. |
| `NAND [reg] [eth]` | bitwise logical NAND | The second register is flipped; its value can be restored with a `NOT` operation. If an immediate value is used, it is flipped at compile time. |
| `OR [reg] [eth]` | bitwise logical OR | - |
| `NOR [reg] [eth]` | bitwise logical NOR | - |
| `XOR [reg] [eth]` | bitwise logical XOR | An extra register will be clobbered; this is true even if an immediate value is used. |
| `NXOR [reg] [eth]` | bitwise logical NXOR | An extra register will be clobbered; this is true even if an immediate value is used. |

### Shift and Rotate Keywords

These keywords simply load the corresponding address from the right and left rotate [lookup tables](#memory-layout).

| Keyword | Description | Notes |
|---------|-------------|-------|
| `ROL [eth]` | rotate left storing the value in **C**  | - |
| `ROR [eth]` | rotate right storing the value in **C** | - |
| `SHL [eth]` | shift left storing the value in **C**   | - |
| `SHR [eth]` | shift right storing the value in **C**  | - |

### Arithmetic Keywords

| Keyword | Description | Notes |
|---------|-------------|-------|
| `ADD [reg] [eth]` | add | All registers will be clobbered; this is true even if an immediate value is used. |
| `SUB [reg] [eth]` | subtract | All registers will be clobbered; this is true even if an immediate value is used. |

### Memory Keywords 🧠

| Keyword | Description | Notes |
|---------|-------------|-------|
| `SET [imm]` | compiles to raw value `[imm]` | - |
| `MOV [reg] [eth]` | copy from second operand to first | - |
| `LOD [add]` | load data from `[add]` into **C** | - |
| `STO [add]` | stores data in **C** at `[add]` | - |

### Jump Keywords 🦘

| Keyword | Description | Notes |
|---------|-------------|-------|
| `PC [add]`  | set program counter to `[add]` | - |
| `LAB [idn]` | define a label pointing to the next statement | - |
| `LIH [con] [add]` | conditional jump if true | All registers will be clobbered, and LIH stands for logic is hard. |

### Miscellaneous Keywords

| Keyword | Description | Notes |
|---------|-------------|-------|
| `NOP` | No operation; used for padding | - |
| `HLT` | halts the CPU until the next interrupt | - |
