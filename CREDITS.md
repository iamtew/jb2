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

Catalog and who paints them: [Typography](docs/md/typography.md). Overlay loads [`webroot/vendor/fonts/overlay.css`](webroot/vendor/fonts/overlay.css). Landing and this docs site load `ui.css` (Better VCR + Geist). Admin keeps its own Geist `@font-face`.

[Geist Sans](https://github.com/vercel/geist-font) variable, v1.8.2, SIL Open Font License 1.1, The Geist Project Authors (Vercel). Vendored as `webroot/vendor/Geist-Variable.woff2` and `docs/vendor/Geist-Variable.woff2`. License text sits next to each file as `Geist.LICENSE`.

| Face | Group | License | Source | Local path |
|---|---|---|---|---|
| Geist | Regular | SIL Open Font License 1.1 | https://vercel.com/font · https://github.com/vercel/geist-font | `webroot/vendor/Geist-Variable.woff2` (docs copy too) |
| Monster Chiller | Display | Foundry license via CDNFonts | https://fonts.cdnfonts.com/css/monster-chiller | [`webroot/vendor/fonts/monster-chiller/`](webroot/vendor/fonts/monster-chiller/) |
| YouMurderer BB | Display | Foundry license via CDNFonts | https://fonts.cdnfonts.com/css/youmurderer-bb | [`webroot/vendor/fonts/youmurderer-bb/`](webroot/vendor/fonts/youmurderer-bb/) |
| Streetmark | Display | Foundry license via CDNFonts | https://fonts.cdnfonts.com/css/streetmark | [`webroot/vendor/fonts/streetmark/`](webroot/vendor/fonts/streetmark/) |
| Germania One | Display | Foundry license via CDNFonts | https://fonts.cdnfonts.com/css/germania-one | [`webroot/vendor/fonts/germania-one/`](webroot/vendor/fonts/germania-one/) |
| Konstruktor | Display | Foundry license via CDNFonts | https://fonts.cdnfonts.com/css/konstruktor | [`webroot/vendor/fonts/konstruktor/`](webroot/vendor/fonts/konstruktor/) |
| Better VCR | Display and Regular | Foundry license via CDNFonts | https://fonts.cdnfonts.com/css/better-vcr | [`webroot/vendor/fonts/better-vcr/`](webroot/vendor/fonts/better-vcr/) (docs copy under `docs/vendor/fonts/better-vcr/`) |
| Flapdoodle | Regular | Foundry license via CDNFonts | https://fonts.cdnfonts.com/css/flapdoodle | [`webroot/vendor/fonts/flapdoodle/`](webroot/vendor/fonts/flapdoodle/) |
| Londrina Solid | Regular | Foundry license via CDNFonts | https://fonts.cdnfonts.com/css/londrina-solid | [`webroot/vendor/fonts/londrina-solid/`](webroot/vendor/fonts/londrina-solid/) |
| Pill Gothic 600mg | Regular | Foundry license via CDNFonts | https://fonts.cdnfonts.com/css/pill-gothic-600mg | [`webroot/vendor/fonts/pill-gothic-600mg/`](webroot/vendor/fonts/pill-gothic-600mg/) |
| Lemondrop | Display | Foundry license via CDNFonts | https://fonts.cdnfonts.com/css/lemondrop | [`webroot/vendor/fonts/lemondrop/`](webroot/vendor/fonts/lemondrop/) |
| Brighton Sans NBP | Regular | Foundry license via CDNFonts | https://fonts.cdnfonts.com/css/brighton-sans-nbp | [`webroot/vendor/fonts/brighton-sans-nbp/`](webroot/vendor/fonts/brighton-sans-nbp/) |

CDNFonts redistributes faces from various foundries. Copies here are for offline OBS and GitHub Pages. If a foundry license is stricter than this project's needs, replace that face. CDNFonts Better VCR (artdzyk) is not the FontStruct face also named Better VCR (Petro Birov, SIL Open Font License 1.1).

## Overlay runtime

Vendored for Overlay and Overlay Stage (`webroot/`). License text sits next to the file.

| File | Version | License |
|---|---|---|
| `webroot/vendor/pixi.min.js` | PixiJS 8.22.0 | MIT, Mathew Groves and Chad Engler. `webroot/vendor/pixi.LICENSE` |

Same bundle as the docs hero. Overlay uses it for a transparent FX layer behind the now-playing widget. The local app home uses it for the idle CRT backdrop.

## Docs site

Vendored for the pages in `docs/`. Not part of the JB2 process. License texts sit next to the files.

| File | Version | License |
|---|---|---|
| `docs/vendor/pixi.min.js` | PixiJS 8.22.0 | MIT, Mathew Groves and Chad Engler. `docs/vendor/pixi.LICENSE` |
| `docs/vendor/marked.min.js` | marked 18.1.2 | MIT, MarkedJS and Christopher Jeffrey. `docs/vendor/marked.LICENSE` |

[PixiJS](https://github.com/pixijs/pixijs). [marked](https://github.com/markedjs/marked).

## Art

`webroot/img/jukeboks.png` is project art (favicon, local landing page, tray and exe icon after `just icon`). `docs/img/jukeboks.png` is a copy for this site. ImageMagick is only the `just icon` tool. It is not shipped.
