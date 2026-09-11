# Development

## Run launcher from repo

```bash
./bin/floresline-launcher
```

## Extension

Edit `extension/floresline-super-launcher@floresline/`, copy to:

```bash
cp -a extension/floresline-super-launcher@floresline \
  ~/.local/share/gnome-shell/extensions/
```

On Wayland, restart the session to reload.

Rebuild zip:

```bash
(cd extension/floresline-super-launcher@floresline && \
  zip -qr ../../packaging/floresline-super-launcher@floresline.shell-extension.zip \
  metadata.json extension.js)
```

## Shell versions

`metadata.json` → `shell-version`: `45`–`50`. Bump when testing newer GNOME.
