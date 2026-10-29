# JB2

Local control plane for [YouTube Music Desktop (pear-desktop)](https://github.com/pear-devs/pear-desktop): Twitch song requests, an OBS overlay, and an admin dock. For [pear-desktop](https://github.com/pear-devs/pear-desktop), not the original [YouTube Music Desktop](https://github.com/th-ch/youtube-music).

Documentation: [docs/md/overview.md](docs/md/overview.md). The same pages are the product site in [`docs/`](docs/index.html) (GitHub Pages from the `/docs` folder on `master`).

## Run

```
just run
just build
just test
just package
```

Flags: `-port`, `-bind`, `-ytmd_host`, `-ytmd_port`, `-webroot`, `-config`.

## Release

Windows zip comes from GitHub Actions (`.github/workflows/dist.yml`). Run it from the Actions tab, or push a numeric tag such as `0.0.1` (no `v` prefix). The workflow does not publish a release. `<version>` is the `version` in `Cargo.toml`.

The `dist` artifact holds `jb2-<version>-win-x64.zip` (`jb2.exe` and `webroot/`). Download it into `dist/` and upload the zip on a GitHub release:

```
gh run download <run-id> -n dist -D dist
```

`just build-dist` on Windows writes the same zip (and the unpacked `dist/` tree).

## License

MIT. See [LICENSE](LICENSE). Third-party software, icons, and fonts: [CREDITS.md](CREDITS.md).
