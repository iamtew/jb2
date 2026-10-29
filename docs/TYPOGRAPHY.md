# Typography

Shared font catalog for the banner overlays. Display faces come first, followed by
regular faces; that order is the persisted numeric index used by `titleFont`,
`messageFont`, `userFont`, and `prefixFont`.

Do not reorder this list unless you also provide URL migration logic. Old meat-bag
bookmarks would otherwise wake up wearing the wrong typeface. New faces are
**appended** so indices `0`–`9` stay stable.

| Index | Font family | Group | Vendored sheet |
|-------|-------------|-------|----------------|
| `0` | `Monster Chiller` | Display | [`vendor/fonts/overlay.css`](../vendor/fonts/overlay.css) |
| `1` | `YouMurderer BB` | Display | same |
| `2` | `Streetmark` | Display | same |
| `3` | `Germania One` | Display | same |
| `4` | `Konstruktor` | Display | same |
| `5` | `Better VCR` | Display & Regular | overlay + [`vendor/fonts/ui.css`](../vendor/fonts/ui.css) |
| `6` | `Flapdoodle` | Regular | overlay |
| `7` | `Londrina Solid` | Regular | overlay |
| `8` | `Pill Gothic 600mg` | Regular | overlay |
| `9` | `Geist` | Regular | overlay + ui (replaces former Segoe UI at this index) |
| `10` | `Lemondrop` | Display | overlay |
| `11` | `Brighton Sans NBP` | Regular | overlay |

**Better VCR** sits in both roles: it is a valid Display face and a valid Regular face.
Overlay pickers always offer the full combined list.

---

## Implementation contract

- Import all twelve faces via [`vendor/fonts/overlay.css`](../vendor/fonts/overlay.css)
  in banner HTML before displaying the font picker.
- Quote family names and provide a generic fallback, for example
  `font-family: 'Better VCR', monospace`.
- Persist the catalog index, not the family name. Current code still accepts legacy
  family-name URLs and rewrites them to indices. Legacy `Segoe UI` names map to
  index `9` (**Geist**).
- The settings menu uses **Better VCR**; overlay fields may use any catalog entry.
- Faces are vendored (see [`CREDITS.md`](../CREDITS.md)). Do not block startup waiting
  for a font; browser fallbacks still render the overlay if a file is missing.

---

## Theme preferred fonts

Each overlay theme declares a preferred **Display** and **Regular** face (see
[`THEMES.md`](THEMES.md)). Apps use those when a font URL param is absent, when a
font control is reset, when the theme changes, and on **Reset all** (for the active
default theme). Meat bags can still pick any catalog face in the Fonts section.

| Field role | Preferred group |
|------------|-----------------|
| Title, prefix, user | Display |
| Message | Regular |

| Theme | Display | Regular |
|-------|---------|---------|
| LCD Glass | `Better VCR` | `Better VCR` |
| Sesh Glass | `Monster Chiller` | `Londrina Solid` |
| Sesh Glass (old) | `Monster Chiller` | `Londrina Solid` |
| 3026 D3C0 | `Lemondrop` | `Brighton Sans NBP` |

Themes register `preferredDisplay` / `preferredRegular` on their
`SeshThemes.catalog` entry (and mirror them in `theme.json`). Host apps resolve
missing/reset fonts from the active theme before falling back to Better VCR.

---

## Related

- [THEMES.md](THEMES.md) — preferred-font fields on theme catalog entries
- [Control_UI.md](Control_UI.md) — settings chrome stays Better VCR
- [`seshbanner/`](../seshbanner/) · [`trbanner/`](../trbanner/) — font pickers + URL indices
