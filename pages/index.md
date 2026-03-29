<head>
  <title>Owen Friedman | Auxv.org</title>
  <meta name="author" content="Owen Friedman">
  <meta name="description" content="I am the slugcat, slayer of dragons, eater of bugs. A self-taught software developer trying to rewrite the world one line at a time.">
</head>

<style>
/* Header */
h1 + blockquote {
  margin-top: -1em;
}

.intro {
  margin-top: 1.2em;
  color: var(--subtle);
  line-height: 1.8;
}

.contact {
  margin-top: 16px;
}

/* Cards */
.card {
  background: var(--surface);
  border-radius: 8px;
  padding: 20px;
  margin-bottom: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin-bottom: 8px;
}

.card-header h1 {
  font-size: 1.3em;
  font-weight: bold;
  margin: 0;
}

.repo-link {
  font-size: 0.95em;
}

.card p {
  color: var(--subtle);
  font-style: italic;
  margin-bottom: 15px;
}

</style>

# Owen Friedman / 5-Pebbles

> I am the slugcat, slayer of dragons, eater of bugs.

<div class="intro">

A hangry(🌮) self-taught software developer trying to rewrite the world one line at a time. Primarily focused on low-level systems development — it's my dream to have a Linux distribution consisting solely of software I have written. This is my website where I can whisper into the void about the things that fascinate me. I hope you enjoy and learn something new!

</div>

<div class="contact">

[me@auxv.org](mailto:me@auxv.org) · [GitHub](https://github.com/5-pebbles)

</div>

## 🏗️ Projects

<div class="card">
<div class="card-header">
<h1>Miros 🌸🌿</h1>
<a class="repo-link" href="https://github.com/5-pebbles/miros">source</a>
</div>

An `ld.so` replacement (dynamic linker/loader, C standard library, and pthreads runtime) that intercepts and redirects symbol resolution for GLIBC-linked programs at runtime.

- [Chapter 1: Frankenstein's Monster 🧟](/projects/miros/frankensteins_monster)
- [Chapter 2: Where to `_start`?](/projects/miros/where_to__start)
- [Chapter ???: Slayer of Dragons, Eater of Bugs 🐔](/projects/miros/slayer_of_dragons_eater_of_bugs) (written out of order)
</div>

<div class="card">
<div class="card-header">
<h1>FRANXX</h1>
<a class="repo-link" href="https://github.com/5-pebbles/franxx">source</a>
</div>

A cute little (split + BLE) computer keyboard I designed & built.

- [Imprecise Instructions Relating to Keyboard Design ⌨️📺🖱️](/projects/franxx/how_not_to_build_a_keyboard)
</div>

<div class="card">
<div class="card-header">
<h1>DIANAC 🧬🧮🏗️</h1>
<a class="repo-link" href="https://github.com/5-pebbles/dianac">source</a>
</div>

A slightly less [esoteric](https://en.wikipedia.org/wiki/Esoteric_programming_language) compiler for my custom esoteric [instruction set architecture](https://en.wikipedia.org/wiki/Instruction_set_architecture).

- [Language Specification 🧬🏗️](/projects/dianac/diana_compiled_language_specification)
<!-- - [An Emulation REPL 🐚](/projects/dianac/an_emulation_repl) -->
<!-- - [Bitwise Logic & My Compiler ⚙️🧮⚡](/projects/dianac/bitwise_logic_and_my_compiler) -->
</div>


## 📚 Articles & Guides

- [My Super Awesome Unobstructed Rectangle Sweep Line Algorithm 📊](/algorithms/unobstructed_sweep_line)
- [Automated SSL/TLS CERTS via Let's Encrypt with Rocket 🔐⬆️⬇️](/projects/auxv-dot-org/lets_encrypt_acme)
- [Theming Your TTY Using Kernel Arguments 🔴 🟢 🔵](/random_crap/ttwhy)
