<head>
  <title>Home | Auxv.org</title>
  <meta name="author" content="Owen Friedman">
  <meta name="description" content="I am the slugcat, slayer of dragons, eater of bugs. A hangry(🌮) self-taught software developer trying to rewrite the world one line at a time.">
</head>

<style>
/* Headings */
#heading {
  text-align: center;
  padding: 0.7em 0 40px 0;
  border-bottom: 2px solid var(--overlay);
}

#heading h1 {
  font-size: 2.5em;
  margin-bottom: 20px;
}

#heading p {
  color: var(--subtle);
  font-size: 1.1em;
  max-width: 700px;
  margin: 0 auto;
  line-height: 1.6;
}

/* Sections */
.section-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin: 50px 0 25px;
  padding-bottom: 12px;
  border-bottom: 2px solid var(--iris);
}

.section-header h2 {
  margin: 0;
}

/* Quick Links */
.quick-links {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 15px;
  margin-bottom: 30px;
}

.quick-link {
  background: var(--surface);
  border: 2px solid var(--surface);
  border-radius: 8px;
  padding: 15px;
  text-align: center;
  transition: all 0.2s;
  text-decoration: none;
  color: var(--text);
}

.quick-link:hover {
  border-color: var(--iris);
  transform: translateY(-2px);
}

a.quick-link::before, a.quick-link::after {
  content: '';
}

/* Cards */
.card {
  background: var(--surface);
  border-radius: 8px;
  padding: 20px;
  margin-bottom: 20px;
}

.card h1 {
  font-size: 1.3em;
  font-weight: bold;
  margin-top: 0;
  margin-bottom: 8px;
  color: var(--iris);
}

.card p {
  color: var(--subtle);
  font-style: italic;
  margin-bottom: 15px;
}
</style>

<div id="heading">

# Welcome to the Auxiliary Vector 🧠

A collection of neurons and synapses flicker to life, within the Laniakea Supercluster. A living catalog of articles, documentation, and accumulated knowledge has finally found a reader.

</div>

<div class="section-header">

## 🗺️ The Basics

</div>

<div class="quick-links">
  <a href="/about" class="quick-link">
    <div style="font-size: 2em; margin-bottom: 5px;">👋</div>
    <strong>About</strong>
  </a>
  <a href="/resume/imperial.pdf" class="quick-link">
    <div style="font-size: 2em; margin-bottom: 5px;">📄</div>
    <strong>Resume</strong>
  </a>
</div>

<div class="section-header">

## 🏗️ Projects

</div>

<div class="card">

# Miros 🌸🌿
A modern ELF interpreter (dynamic linker/loader) written in Rust.
- [Chapter 1: Frankenstein's Monster 🧟](/projects/miros/frankensteins_monster)
- [Chapter 2: Where to `_start`?](/projects/miros/where_to__start)
</div>

<div class="card">

# DIANAC 🧬🧮🏗️
A slightly less [esoteric](https://en.wikipedia.org/wiki/Esoteric_programming_language) compiler for my custom esoteric [instruction set architecture](https://en.wikipedia.org/wiki/Instruction_set_architecture).
- [Language Specification 🧬🏗️](/projects/dianac/diana_compiled_language_specification)
- [An Emulation REPL 🐚](/projects/dianac/an_emulation_repl)
- [Bitwise Logic & My Compiler ⚙️🧮⚡](/projects/dianac/bitwise_logic_and_my_compiler)
</div>

<div class="card">

# FranXX
A cute little (split + BLE) computer keyboard I designed & built.
- [Imprecise Instructions Relating to Keyboard Design ⌨️📺🖱️](/projects/franxx/how_not_to_build_a_keyboard)
</div>

<div class="section-header">

## 📚 Articles & Guides

</div>

- [Automated SSL/TLS CERTS via Let's Encrypt with Rocket 🔐⬆️⬇️](/projects/auxv-dot-org/lets_encrypt_acme)
- [My Super Awesome Unobstructed Rectangle Sweep Line Algorithm 📊](/algorithms/unobstructed_sweep_line)
- [Theming Your TTY Using Kernel Arguments 🔴 🟢 🔵](/random_crap/ttwhy)
