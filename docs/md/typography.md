# Typography

Overlay catalog faces, vendored under `webroot/vendor/fonts/`. Overlay and Overlay Stage load the full list via `webroot/vendor/fonts/overlay.css`. Landing and this docs site load Better VCR + Geist via `ui.css`. Admin stays Geist-only.

Quote family names and give a generic fallback, for example `font-family: 'Better VCR', sans-serif`. Do not block paint waiting for a font; sheets use `font-display: swap`.

Licenses: [Credits](credits.md).

## Catalog

| Family | Group | Sheet | Used by |
|---|---|---|---|
| Monster Chiller | Display | overlay | catalog |
| YouMurderer BB | Display | overlay | catalog |
| Streetmark | Display | overlay | Street |
| Germania One | Display | overlay | Fraktur |
| Konstruktor | Display | overlay | Konstrukt |
| Better VCR | Display and Regular | overlay + ui | LCD Glass, Amber Terminal, landing / Stage / docs headings |
| Flapdoodle | Regular | overlay | Crypt |
| Londrina Solid | Regular | overlay | Poster |
| Pill Gothic 600mg | Regular | overlay | Capsule |
| Geist | Regular | overlay + ui (variable `Geist-Variable.woff2`) | Lower Third, admin, Stage chrome, landing / docs body |
| Lemondrop | Display | overlay | Drop |
| Brighton Sans NBP | Regular | overlay | catalog |

Rows marked **catalog** are for future overlay themes. A theme names a family in `webroot/overlay/style.css`. Overlay Stage has no font selector; type travels with the theme. Theme faces must include `0-9`; the seek times sit in the same family.

## Overlay themes

| Theme | Face |
|---|---|
| LCD Glass | Better VCR |
| Lower Third | Geist |
| Amber Terminal | Better VCR |
| Crypt | Flapdoodle |
| Fraktur | Germania One |
| Konstrukt | Konstruktor |
| Street | Streetmark |
| Drop | Lemondrop |
| Capsule | Pill Gothic 600mg |
| Poster | Londrina Solid |

## Adding a face

1. Put `family.css` and the woff files in `webroot/vendor/fonts/<slug>/`.
2. `@import` that `family.css` from `overlay.css` (and `ui.css` if chrome needs it).
3. Add a row here and in [Credits](credits.md).
4. Name the family on a theme in overlay CSS when you actually use it.

Append. Do not shuffle the table for fun; the list is the tracked catalog.
