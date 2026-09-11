# Rust + GTK4 port (WIP)

Native Rust rewrite of the Floresline launcher using [GTK4](https://www.gtk.org/) via [gtk-rs](https://gtk-rs.org/gtk4-rs/stable/latest/book/).

**Status:** skeleton only. The Python launcher under `bin/` remains the **supported default** until this port reaches feature parity (desktop scan, fuzzy ranking, launch, focus UX).

The GNOME Shell Super-key extension stays **GJS** (`extension/`); it is not part of this crate.

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

## Run the skeleton

```bash
cd rust/floresline-launcher
cargo run
```

Or check without launching a window:

```bash
cargo check
```

If `cargo check` fails with missing `gtk4` / `gtk-4.0` pkg-config errors, install `libgtk-4-dev` (and `pkg-config`) as above.

## Crate layout

```
rust/floresline-launcher/
  Cargo.toml          # package `floresline-launcher`; gtk4 → `gtk`, feature `v4_12`
  src/
    main.rs           # GtkApplication `dev.floresline.Launcher`, UI shell
    desktop.rs        # TODO: scan .desktop dirs
    fuzzy.rs          # TODO: score / rank
    launch.rs         # TODO: exec selected app
```

Bump the `gtk` crate version and/or feature flag (e.g. `v4_14`) in `Cargo.toml` when you need newer GTK APIs. See the gtk-rs book for feature ↔ GTK version mapping.

## Docs

- [GTK](https://www.gtk.org/)
- [GTK4 getting started](https://docs.gtk.org/gtk4/getting_started.html)
- [gtk4-rs book](https://gtk-rs.org/gtk4-rs/stable/latest/book/)
