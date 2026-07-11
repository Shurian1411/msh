# msh

A minimal POSIX-compliant shell, written in Rust.

## Getting started

Simply clone the repo and run it via `cargo`:

```bash
git clone https://github.com/Shurian1411/msh.git
cargo run --release
```

To install it as a standalone binary:

```bash
cargo build --release
cargo install --path .
```
```
```

## Features

It supports auto-completion for both builtin and external commands:

```bash
$ ec<TAB>
echo  ecpg

$ gre<TAB>
grep  gresource
```
```
```

### Builtins

The following builtin commands are supported:

- `cd`
- `complete`
- `echo`
- `exit`
- `history`
- `jobs`
- `pwd`
- `type`
