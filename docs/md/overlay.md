# Overlay

`http://localhost:42420/overlay/`

Use it as an OBS browser source. It shows play/pause state, artist, title, album, and a progress bar. Long lines scroll. The typeface is Better VCR, loaded from CDNFonts. See [Credits](credits.md).

Position comes from JB2. Overlay Stage writes `{ x, y, canvasW, canvasH }`; this page applies it and listens for `event: overlay` on `GET /api/events`. Set the OBS browser source to the same canvas size (1920×1080 or 1280×720). See [Overlay Stage](stage.md).

Playback does not come from JB2. The page opens a WebSocket to [pear-desktop](https://github.com/pear-devs/pear-desktop):

`ws://<host>:26538/api/v1/ws`

`<host>` is the page hostname, unless you override it. JB2 only serves the HTML.

## Query

| Param | Default | Effect |
|---|---|---|
| `host` | page hostname, or `localhost` | pear-desktop host |
| `port` | `26538` | pear-desktop port |
| `pulse` | `3` | progress pulse, clamped to 0–10 |
| `bg` | glass on | `0` or `false` is text only |
| `dev` | off | `true` shows a checkerboard and the raw event log under the glass |

Example: `http://localhost:42420/overlay/?bg=0&pulse=0`
