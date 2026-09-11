# Rust + GTK4 port

Native Rust rewrite of the Floresline launcher using [GTK4](https://www.gtk.org/) via [gtk-rs](https://gtk-rs.org/gtk4-rs/stable/latest/book/).

**Status:** feature-complete — `cargo build --release` produces a working binary with desktop scan, fuzzy ranking, launch, focus UX, CSS theme, and pidfile parity with Python. `install.sh` prefers `rust/floresline-launcher/target/release/floresline-launcher` when present and keeps the Python script as `floresline-launcher-python`.

The GNOME Shell Super-key extension stays **GJS** (`extension/`); it is not part of this crate. Repo `bin/floresline-launcher` remains the Python fallback.

## Ubuntu / Debian build dependencies

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libgtk-4-dev
```

Also need a recent Rust toolchain (`rustup` recommended):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# ensure ~/.cargo/bin is on PATH
```

## Build & install

```bash
cd rust/floresline-launcher
cargo build --release
# then from repo root:
./install.sh
# or manually:
install -m 755 target/release/floresline-launcher ~/.local/bin/floresline-launcher
```

```bash
cargo run --release   # launch once without installing
cargo check           # compile check only
```

If `cargo check` fails with missing `gtk4` / `gtk-4.0` pkg-config errors, install `libgtk-4-dev` (and `pkg-config`) as above.

## Crate layout

```
rust/floresline-launcher/
  Cargo.toml          # package `floresline-launcher`; gtk4 → `gtk`, feature `v4_12`; shlex
  src/
    main.rs           # GtkApplication `dev.floresline.Launcher`, full UI
    desktop.rs        # parse/load .desktop dirs (same filters as Python)
    fuzzy.rs          # score / rank (same algorithm as Python)
    launch.rs         # terminal / gtk-launch / shlex Exec
```

Bump the `gtk` crate version and/or feature flag (e.g. `v4_14`) in `Cargo.toml` when you need newer GTK APIs. See the gtk-rs book for feature ↔ GTK version mapping.

## Docs

- [GTK](https://www.gtk.org/)
- [GTK4 getting started](https://docs.gtk.org/gtk4/getting_started.html)
- [gtk4-rs book](https://gtk-rs.org/gtk4-rs/stable/latest/book/)
