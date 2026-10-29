# Running

```
just run
just build
just test
just package
```

`just run` is `cargo run --release`. `just build` copies `target/release/jb2.exe` to `jb2.exe` in the repo. `just package` writes `dist/jb2.exe` and a copy of `webroot/`. `just build-dist` does that and zips `dist/jb2-<version>-win-x64.zip` using the `version` in `Cargo.toml`. `just icon` rebuilds `webroot/img/jukeboks.ico` from the PNG (needs ImageMagick).

## Release

Windows zip comes from GitHub Actions (`.github/workflows/dist.yml`). Run it from the Actions tab, or push a numeric tag such as `0.0.1` (no `v` prefix). The workflow does not publish a release.

The `dist` artifact is `jb2-<version>-win-x64.zip`. Download it with:

```
gh run download <run-id> -n dist -D dist
```

## Flags

`-port`, `-bind`, `-ytmd_host`, `-ytmd_port`, `-webroot`, `-config`.

`-ytmd_host` and `-ytmd_port` are the [pear-desktop](https://github.com/pear-devs/pear-desktop) local API, not the original [YouTube Music Desktop](https://github.com/th-ch/youtube-music).

| Flag | Default |
|---|---|
| `-port` | `42420` |
| `-bind` | `0.0.0.0` |
| `-ytmd_host` | `localhost` |
| `-ytmd_port` | `26538` |
| `-webroot` | `webroot` next to the exe when that folder exists, otherwise `webroot` in the working directory |
| `-config` | `jb2.sqlite` in that same directory |

The process exits if the port is busy, or if `webroot` is not a directory.

## Config file

`jb2.sqlite` is created if it is missing. If a sibling `jb2.json` exists and the database is empty, that JSON is imported once. The JSON file is left in place.

Admin → Settings can import an old JB1 `jukeboks.json` or a JB2 `jb2.json` later. See [Admin](admin.md).

## Version

`GET /health` returns liveness. `data.version` is `{git tag}-{git hash short}`, plus `-dirty` if the build tree was dirty. Admin → About and the tray **About** item show that string.
