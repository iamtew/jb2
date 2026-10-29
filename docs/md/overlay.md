# Overlay

`http://localhost:42420/overlay/`

Use it as an OBS browser source. It shows play/pause state, artist, title, album, and a progress bar. Long lines scroll. Paused shows **PAUSED** instead of the last track. A down pear-desktop WebSocket shows **NO SIGNAL**.

Look (theme, seek style, opacities, size, pulse, glass) comes from JB2 `overlayLayout`, same as position. Overlay Stage writes it; this page applies `GET /api/overlay` then `event: overlay` on `GET /api/events`. Set the OBS browser source to the same canvas size (1920×1080 or 1280×720). See [Overlay Stage](stage.md).

Playback does not come from JB2. The page opens a WebSocket to [pear-desktop](https://github.com/pear-devs/pear-desktop):

`ws://<host>:26538/api/v1/ws`

`<host>` is the page hostname, unless you override it. JB2 only serves the HTML.

LCD Glass and Amber Terminal use Better VCR. Lower Third uses Geist. Faces are local; see [Typography](typography.md) and [Credits](credits.md).

## Query

| Param | Default | Effect |
|---|---|---|
| `host` | page hostname, or `localhost` | pear-desktop host |
| `port` | `26538` | pear-desktop port |
| `pulse` | from layout (`3`) | progress pulse, clamped to 0–10; set this to override Stage |
| `bg` | from layout | `0` or `false` is text only; set this to override Stage |
| `dev` | off | `true` shows a checkerboard and the raw event log under the panel |

Example: `http://localhost:42420/overlay/?bg=0&pulse=0`
