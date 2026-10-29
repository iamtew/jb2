# Overlay Stage

`http://localhost:42420/stage/`

Composer for placing the now-playing glass on a 16:9 canvas. Open it from the landing page or the tray **Open Overlay Stage**. OBS still uses [`/overlay/`](overlay.md) as the browser source.

JB2 owns the layout (`overlayLayout` in `jb2.sqlite`). Dragging the panel live-updates `/overlay/` over `GET /api/events` (`event: overlay`). Releasing the pointer writes the layout. Stage itself uses placeholder track text and loops play/pause, glow, marquee, and progress so you can see the glass without pear-desktop.

## Canvas

| Size | Pixels |
|---|---|
| 1080p (default) | 1920×1080 |
| 720p | 1280×720 |

The composer scales that canvas to the window. Pointer math stays in canvas pixels. Set the OBS browser source to the same size so the overlay lands where Stage shows it.

Framing lines are thirds (cyan) and center (orange). The panel snaps edges and center to those lines. The grid is Stage-only; it is not drawn in OBS.

## HTTP

| Path | Role |
|---|---|
| `GET /api/overlay` | live layout `{ x, y, canvasW, canvasH }` |
| `PUT /api/overlay` | same fields; `save: true` persists to sqlite. Omit or `false` while dragging. |

A look-and-feel toolbar is not in this page yet. Pulse and glass still come from overlay query params. See [Overlay](overlay.md).
