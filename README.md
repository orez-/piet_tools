# piet_tools

A set of tools for working with [Piet](https://www.dangermouse.net/esoteric/piet.html) code.

## Build

With [Rust and Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) installed:
```bash
cargo build --release
```

This will build the executables to `target/release/`.

## `pieti`

```bash
usage: pieti filename codel-size
```

A Piet interpreter.
Runs the passed image file.

## `pietasm` [beta]

```bash
usage: pietasm build filename codel-size
usage: pietasm run filename codel-size
```

Compiles PietASM to a Piet source image.
`build` will generate the image, `run` will generate and run it.
For more information, see [The PietASM Datasheet](pietasm.md).
