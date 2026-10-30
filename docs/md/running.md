# Running

```
just run
just build
just test
just package
just msi
```

`just run` is `cargo run --release`. `just build` copies `target/release/jb2.exe` to `jb2.exe` in the repo. `just package` writes `dist/jb2.exe` and a copy of `webroot/`. `just build-dist` does that, zips `dist/jb2-<version>-win-x64.zip`, and builds `dist/jb2-<version>-win-x64.msi` using the `version` in `Cargo.toml`. `just msi` is package plus the MSI only; both need WiX 5 on PATH (`dotnet tool install -g wix`). `just icon` rebuilds `webroot/img/jukeboks.ico` from the PNG and the tray menu PNGs from `src/tray_icons/` (needs ImageMagick).

The MSI is per-user: `%LOCALAPPDATA%\Programs\JB2`, no admin. The Start Menu shortcut is a checkbox, on by default. Config (`jb2.sqlite`) lives next to that exe and survives upgrades.

## Release

Windows zip and MSI come from GitHub Actions (`.github/workflows/dist.yml`). Run it from the Actions tab, or push a numeric tag such as `0.0.1` (no `v` prefix). The workflow does not publish a release.

The `dist` artifact is `jb2-<version>-win-x64.zip` and `jb2-<version>-win-x64.msi`. Download it with:

```
gh run download <run-id> -n dist -D dist
```

## Flags

`-port`, `-bind`, `-ytmd_host`, `-ytmd_port`, `-webroot`, `-config`.

`-ytmd_host` and `-ytmd_port` are the [pear-desktop](https://github.com/pear-devs/pear-desktop) local API, not the original [YouTube Music Desktop](https://github.com/th-ch/youtube-music).

The first call to pear-desktop with API auth on pops **Allow** in the player (`id=jb2`). After that, JB2 sends `Authorization: Bearer` on `/api` and overlay/admin open `ws://…/api/v1/ws?token=` using `GET /api/ytmd`. The JWT stays in memory.

| Flag | Default |
|---|---|
| `-port` | `42420` |
| `-bind` | `0.0.0.0` |
| `-ytmd_host` | `localhost` |
| `-ytmd_port` | `26538` |
| `-webroot` | `webroot` next to the exe when that folder exists, otherwise `webroot` in the working directory |
| `-config` | `jb2.sqlite` in that same directory |

The process exits if the port is busy, or if `webroot` is not a directory.

Stderr is `YYYY-MM-DD HH:MM:SS` (local on Windows, UTC elsewhere). Request lines are `METHOD path status` (200, 404, …). Poll GETs stay omitted.

## Config file

`jb2.sqlite` is created if it is missing. If a sibling `jb2.json` exists and the database is empty, that JSON is imported once. The JSON file is left in place.

Admin → Settings can import an old JB1 `jukeboks.json` or a JB2 `jb2.json` later. See [Admin](admin.md).

## Version

`GET /health` returns liveness. `data.version` is `{git tag}-{git hash short}`, plus `-dirty` if the build tree was dirty. Admin → About and the tray **About** item show that string.
