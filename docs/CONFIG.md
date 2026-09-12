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
