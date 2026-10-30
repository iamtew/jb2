# Overlay Stage

`http://localhost:42420/stage/`

Composer for placing the now-playing panel on a 16:9 canvas. Open it from the landing page or the tray **Overlay Stage**. OBS still uses [`/overlay/`](overlay.md) as a browser source. **Copy Overlay URL** on Stage and in the tray copies that URL.

JB2 owns the layout (`overlayLayout` in `jb2.sqlite`). Dragging or resizing the panel live-updates `/overlay/` over `GET /api/events` (`event: overlay`). Releasing the pointer writes the layout. Look settings save immediately. The panel shows the live pear-desktop track (artist, title, album, seek), same as [`/overlay/`](overlay.md): playing, paused, or no signal.

## Canvas

| Size | Pixels |
|---|---|
| 1080p (default) | 1920×1080 |
| 720p | 1280×720 |

The composer scales that canvas to the window. Pointer math stays in canvas pixels. Set the OBS browser source to the same size so the overlay lands where Stage shows it.

The top bar picks 720p/1080p, the framing grid, Snap, and Copy Overlay URL. The left sidebar **Layers** stack is front to back. Click a row to edit that box. Overlay handles are cyan; viz handles are magenta. Grip reorders. Eye hides. Plus adds a visualizer (max 8). Trash removes a visualizer except the last one.

## Look

Theme, seek style, seek direction, playing/paused opacity, pulse, glass, and the spectrum layers. The framing grid sits in the top bar. Each overlay look control has a refresh back to default, plus reset size. Hide Overlay with the eye to grey those controls. Hide a visualizer the same way; its style controls grey out. **Output** (shared) picks the playback device (`default` or a listed output name). The line under that select is the capture status. **Smoothing** is the capture release (0–1), also shared. Any unhidden visualizer starts Windows loopback capture (`audioVisualizer.enabled`). **Style** is Bands, Wave, Dots, Trail, or Warp. **Bands** is 1–32 (resampled from the capture). **Color** follows the seek gradient or picks one of those gradients. **Direction** is left→right, right→left, bottom→top, or top→bottom (`colorDir`). **Mirror** reflects bands, the wave, the dots, Trail, and Warp across the midline. **Peak fall** draws a cap that snaps up and drops; 0 hides it, higher drops faster. On Trail it is wake length (0 is the live line only). Each of those has a refresh control back to default (bands style, 16 bands, follow seek, left→right, smoothing 0.35, mirror off, peak fall 0.35). Each visualizer is its own box (`vizLayers[]` `x` `y` `w` `h`) with the same corner and edge handles as the panel. OBS follows the stack. Dragging the panel does not move them. Resize **scales** the whole panel from a 500px design width so type, padding, and the seek bar stay in proportion. Themes change look only (colors, type, fill, and a PixiJS FX halo); the widget structure stays artist / title / album / seek bar. Glass off hides that FX.

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

Seek styles: Sesh Sofa (yellow/red), Ice, Magenta, White, Sunset, Forest, Gold, Lava, Ocean, Candy, Ultraviolet, Acid, Copper, Cherry, Vapor, Graphite, Honey. **Seek direction** is left→right, right→left, bottom→top, or top→bottom (`seekDir`, default `lr`). White is a solid fill, so direction does nothing.

Framing grids are Stage-only (not drawn in OBS). Toggle any mix of thirds, center, golden, broadcast safe (action ~90% / title ~80%), and 4×4; uncheck all for off. With **Snap** on (default), the active box snaps to the union of selected grid lines and to the other box’s edges and center. Uncheck Snap to drag freely. `grid` is a comma-separated list (`center,thirds` default) or `off`. `snap` is a boolean (default true). Legacy `thirds-center` still loads as thirds + center.

## HTTP

| Path | Role |
|---|---|
| `GET /api/overlay` | live layout |
| `PUT /api/overlay` | same fields; `save: true` persists to sqlite. Omit or `false` while dragging. |

Layout fields: `{ x, y, canvasW, canvasH, theme, seekStyle, seekDir, playingOpacity, pausedOpacity, panelW, panelH, pulse, glass, grid, snap, overlayHidden, layers, vizLayers }`. `grid` is a sorted comma list of `thirds`, `center`, `golden`, `safe`, `quad`, or `off`. `snap` is Stage-only (OBS ignores it). `panelW` is the visual width; the widget is always 500px internally and scaled. `panelH` is unused for paint (height follows content × scale). `layers` is front-to-back ids (`overlay` plus viz ids). `vizLayers` is 1–8 spectrum boxes. Overlay query `?pulse=` and `?bg=0` still override pulse and glass when present.
