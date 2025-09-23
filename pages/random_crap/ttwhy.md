<head>
  <title> TT-Why? 🖨️ | Auxv.org </title>
  <meta name="author" content="Owen Friedman">
</head>

# Theming Your Linux TTY Using Kernel Arguments | TT-Why? 🖨️

I really don't like the default theme of the Linux TTY (🤮)... I've seen some blog posts about using terminal escape codes in `/etc/issues` and `~/.bashrc`, but this leaves the default theme around for the startup log messages.

<br/>

If only the Linux kernel developers had made some kernel arguments just for this... (they did).

<br/>
<details>
<summary><b>Table of Contents:</b></summary>

- [vt.default_(red 🔴 | grn 🟢 | blu 🔵)](#vtdefaultred---grn---blu) \
  [Catppuccin](#catppuccin)
- [How to Pass Kernel Arguments](#how-to-pass-kernel-arguments)
  1. [EFI boot stub](#1-efi-boot-stub)

</details>


## vt.default_(red 🔴 | grn 🟢 | blu 🔵)

The kernel arguments in question are: [`vt.default_red`, `vt.default_grn`, and `vt.default_blu`], each takes an array of 16 comma-separated 8-bit (0-255) decimal numbers representing the terminal colors.

<br/>

The 16 colors correspond to the standard terminal color palette:

| Id | Color | Id | Color | Id | Color | Id | Color |
|:---|:------|:---|:------|:---|:------|:---|:------|
| 0 | **Black**  | 4 | **Blue**    | 8  | **Bright Black**  | 12 | **Bright Blue**    |
| 1 | **Red**    | 5 | **Magenta** | 9  | **Bright Red**    | 13 | **Bright Magenta** |
| 2 | **Green**  | 6 | **Cyan**    | 10 | **Bright Green**  | 14 | **Bright Cyan**    |
| 3 | **Yellow** | 7 | **White**   | 11 | **Bright Yellow** | 15 | **Bright White**   |

The kernel indexes each array with the `Id` to set the RGB values for each color in the palette.

<br/>
<details>
<summary id="catppuccin"><b>Catppuccin</b></summary>

```
vt.default_red=36,237,166,238,138,245,139,184,91,237,166,238,138,245,139,165
vt.default_grn=39,135,218,212,173,189,213,192,96,135,218,212,173,189,213,173
vt.default_blu=58,150,149,159,244,230,202,224,120,150,149,159,244,230,202,203
```

</details>

<!-- <details> -->
<!-- <summary><b>Rosé Pine</b></summary> -->

<!-- ``` -->
<!-- vt.default_red=25,31,38,110,144,224,224,82,235,246,235,49,156,196,246,82 -->
<!-- vt.default_grn=23,29,35,106,140,222,222,79,111,193,188,116,207,167,193,79 -->
<!-- vt.default_blu=36,46,58,134,170,244,244,103,146,119,186,143,216,231,119,103 -->
<!-- ``` -->

<!-- </details> -->

## How to Pass Kernel Arguments

Select the instructions that work for your bootloader/system:

### 1. EFI boot stub

If you're using direct EFI boot, add the arguments to your `efibootmgr` command:

```sh
efibootmgr --create --disk /dev/sdX --part Y \
  --label "Linux" \
  --loader '\vmlinuz-linux' \
  --unicode 'root=UUID=your_uuid initrd=\initramfs-linux.img another_argument=another_value'
```


<br/>
And with that you can make your TTY as <u>fabulous</u> as 16-color bitmap fonts can be, enjoy!
