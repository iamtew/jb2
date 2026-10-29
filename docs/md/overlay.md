# Overlay

`http://localhost:42420/overlay/`

Use it as an OBS browser source. It shows play/pause state, artist, title, album, and a progress bar. Long lines scroll. Paused shows centered `Jukeboks Paused` / `Add song with !sr <song / URL>` instead of the last track. A down pear-desktop WebSocket shows **NO SIGNAL**.

Look (theme, seek style, opacities, size, pulse, glass) comes from JB2 `overlayLayout`, same as position. Overlay Stage writes it; this page applies `GET /api/overlay` then `event: overlay` on `GET /api/events`. Set the OBS browser source to the same canvas size (1920×1080 or 1280×720). See [Overlay Stage](stage.md).

Each theme paints a small PixiJS FX layer behind the widget (scan, motes, neon, and so on). Type and the seek bar stay HTML. Text-only (`glass` off or `?bg=0`) hides the FX. If WebGL fails, the canvas is removed and CSS look still works.

A second Pixi layer, `.viz-layer`, sits on the canvas under the panel. It draws spectrum bars from `GET /sse/audio` in the theme colors. Overlay Stage owns the on/off switch (`audioVisualizer.enabled`). This page follows `event: audioVisualizer` on `GET /api/events`. Capture errors and the disabled state hide the layer. The panel does not move the bars.

Playback does not come from JB2. The page opens a WebSocket to [pear-desktop](https://github.com/pear-devs/pear-desktop):

`ws://<host>:26538/api/v1/ws?token=` (token from JB2 `GET /api/ytmd`)

`<host>` is the page hostname, unless you override it. JB2 only serves the HTML.

LCD Glass and Amber Terminal use Better VCR. Lower Third uses Geist. Crypt, Fraktur, Konstrukt, Street, Drop, Capsule, and Poster each pick a catalog face. Faces are local; see [Typography](typography.md) and [Credits](credits.md).

## Query

| Param | Default | Effect |
|---|---|---|
| `host` | page hostname, or `localhost` | pear-desktop host |
| `port` | `26538` | pear-desktop port |
| `pulse` | from layout (`3`) | progress pulse, clamped to 0–10; set this to override Stage |
| `bg` | from layout | `0` or `false` is text only; set this to override Stage |
| `dev` | off | `true` shows a checkerboard and the raw event log under the panel |

Example: `http://localhost:42420/overlay/?bg=0&pulse=0`
