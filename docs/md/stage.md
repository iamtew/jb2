# Overlay Stage

`http://localhost:42420/stage/`

Composer for placing the now-playing panel on a 16:9 canvas. Open it from the landing page or the tray **Open Overlay Stage**. OBS still uses [`/overlay/`](overlay.md) as a browser source. **Copy Overlay URL** on Stage and in the tray copies that URL.

JB2 owns the layout (`overlayLayout` in `jb2.sqlite`). Dragging or resizing the panel live-updates `/overlay/` over `GET /api/events` (`event: overlay`). Releasing the pointer writes the layout. Look settings save immediately. Stage uses placeholder track text; the preview button cycles play, pause, no-signal, and an 8s auto loop.

## Canvas

| Size | Pixels |
|---|---|
| 1080p (default) | 1920×1080 |
| 720p | 1280×720 |

The composer scales that canvas to the window. Pointer math stays in canvas pixels. Set the OBS browser source to the same size so the overlay lands where Stage shows it.

The top bar picks 720p/1080p and preview. Look settings sit in the left sidebar.

## Look

Theme, seek style, playing/paused opacity, pulse, glass, framing grid, and the spectrum layer. Each look control except the spectrum checkbox has a refresh control back to default, plus reset size. The Visualizer checkbox writes `audioVisualizer.enabled` and starts Windows loopback capture. Bars fill the canvas under the panel and under the framing grid. Dragging the panel does not move them. OBS [`/overlay/`](overlay.md) shows the same layer. Resize **scales** the whole panel from a 500px design width so type, padding, and the seek bar stay in proportion. Themes change look only (colors, type, fill, and a PixiJS FX halo); the widget structure stays artist / title / album / seek bar. Glass off hides that FX.

| Theme | Look |
|---|---|
| LCD Glass | Translucent blue glass, Better VCR |
| Lower Third | Opaque charcoal chyron, Geist |
| Amber Terminal | Black slab, amber Better VCR |
| Crypt | Black / blood, Flapdoodle, red bloom |
| Fraktur | Parchment, Germania One, gold ember |
| Konstrukt | Black slab, red bar, Konstruktor flash |
| Street | Asphalt, Streetmark, neon outline |
| Drop | Candy glass, Lemondrop, pastel bloom |
| Capsule | Mint pill, Pill Gothic 600mg, inner glow |
| Poster | Orange / cyan slab, Londrina Solid |

Typefaces come with the theme ([Typography](typography.md)). Stage does not pick fonts.

Seek styles: Sesh Sofa (yellow/red), Ice, Magenta, White, Sunset, Forest, Gold, Lava, Ocean, Candy, Ultraviolet, Acid, Copper, Cherry, Vapor, Graphite, Honey.

Framing grids are Stage-only (not drawn in OBS). Toggle any mix of thirds, center, golden, broadcast safe (action ~90% / title ~80%), and 4×4; uncheck all for off. The panel snaps to the union of selected lines. `grid` is a comma-separated list (`center,thirds` default) or `off`. Legacy `thirds-center` still loads as thirds + center.

## HTTP

| Path | Role |
|---|---|
| `GET /api/overlay` | live layout |
| `PUT /api/overlay` | same fields; `save: true` persists to sqlite. Omit or `false` while dragging. |

Layout fields: `{ x, y, canvasW, canvasH, theme, seekStyle, playingOpacity, pausedOpacity, panelW, panelH, pulse, glass, grid }`. `grid` is a sorted comma list of `thirds`, `center`, `golden`, `safe`, `quad`, or `off`. `panelW` is the visual width; the widget is always 500px internally and scaled. `panelH` is unused for paint (height follows content × scale). Overlay query `?pulse=` and `?bg=0` still override pulse and glass when present.
