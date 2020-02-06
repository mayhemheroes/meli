# meli
For a quick start, build and install locally:

```sh
 PREFIX=~/.local make install
```

Available subcommands:
 - meli (builds meli with optimizations in `$CARGO_TARGET_DIR`)
 - install (installs binary in `$BINDIR` and documentation to `$MANDIR`)
 - uninstall
Secondary subcommands:
 - clean (cleans build artifacts)
 - check-deps (checks dependencies)
 - install-bin (installs binary to `$BINDIR`)
 - install-doc (installs manpages to `$MANDIR`)
 - help (prints this information)
 - dist (creates release tarball named `meli-VERSION.tar.gz` in this directory)
 - deb-dist (builds debian package in the parent directory)
 - distclean (cleans distribution build artifacts)

The Makefile *should* be portable and not require a specific `make` version.

# Documentation

After installing meli, see `meli(1)` and `meli.conf(5)` for documentation.

# Building

meli requires rust 1.39 and rust's package manager, Cargo. Information on how
to get it on your system can be found here: <https://doc.rust-lang.org/cargo/getting-started/installation.html>

With Cargo available, the project can be built with

```sh
make meli
```

The resulting binary will then be found under `target/release/meli`

Run:

```sh
make install
```

to install the binary and man pages. This requires root, so I suggest you override the default paths and install it in your `$HOME`:

```sh
make PREFIX=$HOME/.local install
```

See `meli(1)` and `meli.conf(5)` for documentation.

You can build and run meli with one command:

```sh
cargo run --release
```

While the project is in early development, meli will only be developed for the
linux kernel and respected linux distributions. Support for more UNIX-like OSes
is on the roadmap.

# Building in Debian

Building with Debian's packaged cargo might require the installation of these
two packages: `librust-openssl-sys-dev librust-libdbus-sys-dev`

A `*.deb` package can be built with `make deb-dist`

# Building with notmuch

To use the optional notmuch backend feature, you must have `libnotmuch` installed in your system. In Debian-like systems, install the `libnotmuch5 libnotmuch-dev` packages.

To build with notmuch support, prepend the environment variable `MELI_FEATURES='notmuch'` to your make invocation:

```sh
MELI_FEATURES="notmuch" make
```

or if building directly with cargo, use the flag `--features="notmuch"'.

# Building with JMAP

To build with JMAP support, prepend the environment variable `MELI_FEATURES='jmap'` to your make invocation:

```sh
MELI_FEATURES="jmap" make
```

or if building directly with cargo, use the flag `--features="jmap"'.

# Development

Development builds can be built and/or run with

```
cargo build
cargo run
```

There is a debug/tracing log feature that can be enabled by using the flag
`--feature debug-tracing` after uncommenting the features in `Cargo.toml`. The logs
are printed in stderr, thus you can run meli with a redirection (i.e `2> log`)

Code style follows the default rustfmt profile.

# Configuration

meli by default looks for a configuration file in this location: `$XDG_CONFIG_HOME/meli/config.toml`

You can run meli with arbitrary configuration files by setting the `$MELI_CONFIG`
environment variable to their locations, ie:

```sh
MELI_CONFIG=./test_config cargo run
```

# Testing

How to run specific tests:

```sh
cargo test -p {melib, meli} (-- --nocapture) (--test test_name)
```

# Profiling

```sh
perf record -g target/debug/bin
perf script | stackcollapse-perf | rust-unmangle | flamegraph > perf.svg
```