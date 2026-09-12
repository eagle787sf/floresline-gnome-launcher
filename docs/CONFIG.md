# Configuration

Floresline keeps settings in plain TOML — no settings GUI (by design: lean and stable).

## Files

| File | Purpose |
|------|---------|
| `~/.config/floresline-launcher/config.toml` | Shortcuts + launcher options |
| `~/.config/floresline-launcher/extras.toml` | Private custom commands (`mtb`, etc.) — not in git |
| `config/config.example.toml` (repo) | Template `install.sh` copies on first install |

## Quick start

```bash
cp config/config.example.toml ~/.config/floresline-launcher/config.toml
# edit preferred_binding, max_results, etc.
./install.sh   # applies GNOME keybindings + extension on/off from config
```

## Recommended default

`preferred_binding = "Super+backslash"` — avoids bare Super fighting GNOME Activities.

Set `enable_super_alone_extension = true` only if you want Pop-style Super alone (then log out/in once on Wayland).

## Options the Rust app reads live

- `max_results` — rows shown (1–50)
- `hint` — bottom hint text

Restart the launcher (or toggle it) after editing those. Shortcut / extension flags need `./install.sh` again.

## Super+\\ without Activities stealing focus

Set `disable_bare_super_overview = true` (default). `install.sh` clears `org.gnome.mutter overlay-key` so tapping Super no longer opens Activities / GNOME search.

Open Activities with **Super+Shift+Space** (or **Super+\\`** / Above_Tab) instead.

If bare Super still opens Overview, run `./install.sh` again or:
```bash
gsettings set org.gnome.mutter overlay-key ''
```
