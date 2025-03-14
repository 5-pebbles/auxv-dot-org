<head>
  <title>The Three Musketeers | Auxv.org</title>
  <meta name="author" content="Owen Friedman">
  <meta name="description" content="How to retrieve command line arguments and environment variables from the stack in x86_64 assembly 🥞🧰...">
</head>

# The Three Musketeers 👨‍👨‍👦

If you're anything like me, you have wondered how a program gets its command line arguments and environment variables. This guide will explain how to write a simple program that fetches them directly from the initial stack on startup.

<br/>

In case you aren't like me, here is brief introduction to these three data structures:

1. **Arguments:** Are values passed to your program from the command line. For example if you run `git commit` there are two arguments `git` and `commit`.

2. **Environment Variables:** Are key-value pairs that provide context to the running program. They're commonly used for configuration and system information. For example, the string `HOME=/home/ghostbird` stores the path to your home directory.

3. **The Auxiliary Vector:** Is more obscure and this website's namesake. It's an assortment of possibly useful information passed form the Linux kernel to the runtime and/or dynamic linker.

<br/>
<details>
<summary><b>Table of Contents:</b></summary>

- [The Three Musketeers 👨‍👨‍👦](#the-three-musketeers)
- [The Secret Sauce: What's on the Stack? 🥞](#the-secret-sauce-whats-on-the-stack)
- [Some Assembly Required 🧰](#some-assembly-required)
  - [The `x86_64` register naming "convention"](#the--register-naming-convention)
  - [Convert a number to a string: `number_to_string`](#convert-a-number-to-a-string)
  - [Print a string to stdout: `println`](#print-a-string-to-stdout)
  - [Building and Running 🏗️](#building-and-running)
- [The Complete Program 🏁](#the-complete-program)

</details>


## The Secret Sauce: What's on the Stack? 🥞

The stack is conceptually an area of memory that is used for a first in last out data-structure. In `x86_64` Linux the top most address is stored in the `rsp` register. When your program starts the Linux kernel pushes three sub-data-structures onto the stack... arguments, environment variables, and the auxiliary vector.

<br/>

Here is a diagram to demonstrate its layout in memory:

```x86asm
; + Newly Pushed Vaules      Examples:               |-----------------|
; |-------------------|    |----------------|  |---> | "/bin/git", 0x0 |
; | Arg Count         |    | 2              |  |     |-----------------|
; |-------------------|    |----------------|  |
; | Arg Pointers...   |    | Pointer,       | -|   |---------------|
; |                   |    | Other Pointer  | ---> | "commit", 0x0 |
; |-------------------|    |----------------|      |---------------|
; | Null              |    | 0x0            |
; |-------------------|    |----------------|       |-----------------------------|
; | Env Pointers...   |    | Pointer,       | ----> | "HOME=/home/ghostbird", 0x0 |
; |                   |    | Other Pointer  | ---|  |-----------------------------|
; |-------------------|    |----------------|    |   
; | Null              |    | 0x0            |    |   |---------------------------|
; |-------------------|    |----------------|    |-> | "PATH=/bin:/usr/bin", 0x0 |
; | Auxv Type...      |    | AT_RANDOM      |        |---------------------------|
; | Auxv Vaule...     |    | Union->Pointer | -|
; |-------------------|    |----------------|  |   |---------------------------|
; | AT_NULL Auxv Pair |    | AT_NULL (0x0)  |  |-> | [16-bytes of random data] |
; |-------------------|    | Undefined      |      |---------------------------|
;                          |----------------|
```

On `x86_64` the stack grows down, and so we access each field by adding to the value stored in the `rsp` register, then dereferencing it as a pointer.


## Some Assembly Required 🧰

The examples I use will be in Assembly which means we need to implement a few simple helper functions before we start.

### The `x86_64` register naming "convention"

My implementation involves sub-registers, so here's a helpful and snarky reference table:

| Name  |    Bits   | Meaning |
|:-----:|:---------:|:-------:|
| `al`  | bits 0-7  | Accumulator low |
| `ah`  | bits 8-15 | Accumulator high (the sequel) |
| `ax`  | bits 0-15 | Accumulator eXtended (marketing department got involved) |
| `eax` | bits 0-31 | Extended Accumulator eXtended (we heard you like extensions) |
| `rax` | bits 0-63 | Register Accumulator eXtended (running out of letters here) |

> <b style="color: var(--love);">Hot take:</b> They really dropped the ball on naming the 64-bit registers. It should've been xeax (eXtended Extended Accumulator eXtended) for maximum confusion. The 128-bit register could still be exeax though...


### Convert a number to a string: `number_to_string`

```x86asm
section .text
    global number_to_string

;; Converts a number (1-999) to a null-terminated string.
;; @arguments (number: rdi)
;; @clobbers [rax]
;; @returns (pointer_to_string: rax) WARN: Lives until next call...
number_to_string:
    push rdi

    mov rax, rdi        ; Number to convert
    lea rdi, [buffer]   ; Get buffer address
    
    ; Get last digit
    xor rdx, rdx
    div 10
    add dl, '0'
    mov [rdi + 2], dl  ; Store in buffer
    
    ; Get middle digit
    xor rdx, rdx
    div 10
    add dl, '0'
    mov [rdi + 1], dl
    
    ; Get first digit
    add al, '0'        ; Last division result is in al
    mov [rdi], al
    
    mov rax, rdi       ; Return buffer pointer

    pop rdi

    ret

section .data
buffer: times 4 db 0   ; Buffer for 3 digits + null
```

A few things to note: 
- The `xor rdx, rdx` instruction is a fast way to zero out the `rdx` register.
- The `div` instruction divides `rax` by the operand and stores the remainder in `rdx`.
- The code points for ASCII digits are continuous, so adding `'0'` to the lowest 8-bits of our remainder converts it to the corresponding character.

### Print a string to stdout: `println`

```x86asm
section: .data
    newline db 10 ; Newline character
section .text
    global println

;; Writes the null-terminated string to `stdout` followed by a newline.
;; @arguments (pointer_to_string: rsi)
;; @clobbers [rcx, rdx, rsi, r11]
println:
; Find the length of the string
mov rdx, -1
.loop:
inc rdx ; Incrment rdx
cmp byte [rsi + rdx], 0 
jne .loop ; Loop again if not equal

; Write Linux system call
mov rax, 1 ; The linux system call table index for write
mov rdi, 1 ; The file descriptor (1 is stdout)
; The string pointer is already in rsi and the length is already in rdx
syscall

; Now we just need to write the newline
mov rax, 1
mov rdi, 1
mov rsi, newline
mov rdx, 1
syscall

ret
```

The `syscall` instruction lets user-space programs interact with the kernel. It triggers an interrupt, causing the CPU to jump to the kernel's handler to perform the requested operation before returning control.


### Building and Running 🏗️

Let's create a simple "Hello World" program in `main.asm`:

```x86asm
section .data
    hello_world db "Hello World!", 0 ; Null-terminated
section .text
    global _start
    extern println

_start:
; Print "Hello World!"
mov rsi, hello_world
call println

; Exit with 0 status code
mov rax, 60 ; Exit syscall index
mov rdi, 0
syscall
```

To make these files into one executable we assemble them into elf64 object files and then link them together using ld:

```sh
mkdir target
# Compile our helper functions
nasm -f elf64 "number_to_string.asm" -o "target/number_to_string.o"
nasm -f elf64 "println.asm" -o "target/println.o"
# Compile our main program
nasm -f elf64 "main.asm" -o "target/main.o"
# Link everything together and set the entry to `_start`
ld -o target/three_musketeers target/*.o -e _start
```

Now if you run it we should see this:

```
❯ ./target/three_musketeers
Hello World!
```

## The Complete Program 🏁

Here's the final program that iterates through each data structure and prints it to stdout:

```x86asm
section .text
    global _start
    extern println
    extern number_to_string

_start:
    and rsp, -16
    lea rbx, [rsp]
; Print the number of arguments 📢
process_arguments_length:
    mov rdi, [rbx]
    call number_to_string
    mov rdi, rax
    call println

    add rdi, 8
; Print all arguments 🤬
process_arguments:
    mov rdi, [rbx]
    call println
    add rbx, 8
    cmp qword [rbx], 0
    jne process_arguments

    add rbx, 8 ; Skip the null-terminator
; Print all environment variables 🌱
process_environment:
    mov rdi, [rbx]
    call println
    add rbx, 8
    cmp qword [rbx], 0
    jne process_environment

    add rbx, 8 ; Skip the null-terminator
; Print auxiliary vector types 🗄️
process_auxiliary_vector:
    mov rdi, [rbx]
    call number_to_string
    mov rdi, rax
    call println
    add rbx, 16
    cmp qword [rbx], 0
    jne process_auxiliary_vector

linux_exit_via_syscall:
    mov rax, 60
    xor rdi, rdi ; Exit normally with status 0
    syscall
```
