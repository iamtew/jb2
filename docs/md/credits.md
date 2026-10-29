# Credits

JB2 itself is MIT. The license text is `LICENSE` at the repository root.

This file names third-party software, icons, and fonts the project ships or loads. Direct dependencies only. The rest of the Rust graph is in `Cargo.lock`.

The same text is in `docs/md/credits.md` so the docs site can render it. GitHub Pages publishes the `docs/` folder only.

## Rust

Compiled into `jb2`. Versions are the ones locked in `Cargo.lock`.

| Crate | Version | License | Role |
|---|---|---|---|
| tiny_http | 0.12.0 | MIT OR Apache-2.0 | HTTP server |
| rusqlite | 0.32.1 | MIT | `jb2.sqlite` |
| libsqlite3-sys | 0.30.1 | MIT | SQLite binding used by rusqlite |
| SQLite | bundled | Public Domain | amalgamation compiled in by rusqlite `bundled` |
| ureq | 2.12.1 | MIT OR Apache-2.0 | outbound HTTP (pear-desktop, Twitch, oEmbed) |
| native-tls | 0.2.18 | MIT OR Apache-2.0 | TLS for ureq |
| serde | 1.0.229 | MIT OR Apache-2.0 | serialization |
| serde_json | 1.0.154 | MIT OR Apache-2.0 | JSON |
| url | 2.5.8 | MIT OR Apache-2.0 | URL parsing |
| sha2 | 0.10.9 | MIT OR Apache-2.0 | Twitch PKCE S256 challenge |
| windows | 0.58.0 | MIT OR Apache-2.0 | tray and console on Windows builds |
| winres | 0.1.12 | MIT | Windows exe icon at build time |

`windows` is a Windows-only dependency. `winres` is a build dependency.

## Icons

[Tabler Icons](https://github.com/tabler/tabler-icons) outline set, MIT, Paweł Kuna, v3.34.1. Paths are copied into `webroot/admin/index.html` and `docs/icons.svg`. There is no separate icon font in the tree.

## Fonts

Better VCR is not in the repo. The landing page and the OBS overlay load it at runtime from `https://fonts.cdnfonts.com/css/better-vcr` (`webroot/style.css`, `webroot/overlay/style.css`).

[CDNFonts](https://www.cdnfonts.com/better-vcr.font) lists that face as designed by artdzyk and “Free for personal use”, and tells you to research the license before using it. That remote file is not the FontStruct face also named Better VCR (Petro Birov, SIL Open Font License 1.1). JB2 does not vendor either file.

The admin dock and this docs site use `system-ui`. They do not load Better VCR.

## Docs site

Vendored for the pages in `docs/`. Not part of the JB2 process. License texts sit next to the files.

| File | Version | License |
|---|---|---|
| `docs/vendor/pixi.min.js` | PixiJS 8.22.0 | MIT, Mathew Groves and Chad Engler. `docs/vendor/pixi.LICENSE` |
| `docs/vendor/marked.min.js` | marked 18.1.2 | MIT, MarkedJS and Christopher Jeffrey. `docs/vendor/marked.LICENSE` |

[PixiJS](https://github.com/pixijs/pixijs). [marked](https://github.com/markedjs/marked).

## Art

`webroot/img/jukeboks.png` is project art (favicon, local landing page, tray and exe icon after `just icon`). `docs/img/jukeboks.png` is a copy for this site. ImageMagick is only the `just icon` tool. It is not shipped.
