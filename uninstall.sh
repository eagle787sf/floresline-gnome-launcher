#!/usr/bin/env bash
set -euo pipefail
EXT_UUID="floresline-super-launcher@floresline"
rm -f "$HOME/.local/bin/floresline-launcher" "$HOME/.local/bin/floresline-launcher-toggle"
rm -f "$HOME/.local/share/applications/floresline-launcher.desktop"
rm -rf "$HOME/.local/share/gnome-shell/extensions/${EXT_UUID}"
python3 - << 'PY'
import subprocess, ast
raw = subprocess.check_output(["gsettings", "get", "org.gnome.shell", "enabled-extensions"], text=True).strip()
try:
    exts = [e for e in ast.literal_eval(raw) if e != "floresline-super-launcher@floresline"]
except Exception:
    exts = []
fmt = "[" + ", ".join(f"'{e}'" for e in exts) + "]"
subprocess.check_call(["gsettings", "set", "org.gnome.shell", "enabled-extensions", fmt])
print("Removed extension from enabled list. Log out/in to finish.")
PY
