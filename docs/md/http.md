# HTTP

Responses: `{ "exitCode": 0|1, "message"?, "data"? }`. App failures are HTTP 200 with `exitCode: 1`. A pear-desktop hard fail is 502. Unknown `/api/*` and `/cmd/jb/*` routes are 404.

No auth, so the LAN (OBS, Streamer.bot on another PC) can reach everything. Browser writes (`POST`, `PUT`, `PATCH`, `DELETE`) whose `Origin` does not match the `Host` they were sent to get 403, so a random web page cannot change config or chat as the bot. `Origin: null` never matches. Clients that send no `Origin` (Streamer.bot, curl) are unaffected. Request bodies over 1 MiB get 413.

| Path | Role |
|---|---|
| `GET /health` | liveness (`data.version` is `{git tag}-{git hash short}`, plus `-dirty` if the build tree was dirty) |
| `GET /cmd/jb/songinfo` | now playing |
| `GET /cmd/jb/queueinfo` | upcoming queue (fails if paused) |
| `GET /cmd/jb/songrequest?input=` | search and queue (`now=1` plays immediately) |
| `GET /cmd/jb/search?input=` | search hits (`browse=` lists album, artist top songs, or playlist tracks) |
| `GET /cmd/jb/commands` | Twitch chat commands on/off |
| `GET /cmd/jb/commands/on` `off` | enable or disable Twitch chat commands |
| `GET /cmd/jb/play` `pause` `next` `previous` | transport aliases (`GET` or `POST`) |
| `/cmd/ytmd/*` | pear-desktop proxy (method suffix or `?_method=`) |
| `GET /api/events` | SSE (`twitch` / `chat` / `seed` / `commands` / `cmdHistory` / `queue` / `overlay` / `audioVisualizer`) |
| `GET /api/queue` | admin queue rows |
| `/api/config` | blacklist, maxDuration, Twitch, command cooldowns, overlayLayout (`GET`, and `POST` or `PUT` to save) |
| `GET /api/ytmd` | pear-desktop JWT for overlay/admin WebSocket (`data.accessToken`) |
| `GET` / `PUT /api/overlay` | live overlay layout (`x`, `y`, `canvasW`, `canvasH`, `theme`, `seekStyle`, opacities, `panelW`/`panelH`, `pulse`, `glass`, `grid` comma list or `off`, `vizX`/`vizY`/`vizW`/`vizH`, `layers` front-to-back `overlay,viz`); `save: true` on PUT writes sqlite. See [Overlay Stage](stage.md) |
| `GET /sse/audio` | audio analysis SSE (`event: audio`). Frames are `{ "type": "audio", "volume", "bass", "mid", "treble", "spectrum", "beat", "beatStrength" }` plus `volumeLeft` / `volumeRight` when the device is stereo. Status frames are `{ "type": "status", "state": "capturing" \| "silent" \| "error" \| "device_changed" }`. Not mixed into `/api/events` |
| `GET /api/audio` | one snapshot of the latest analysis frame or status |
| `GET /api/audio/devices` | output devices for the visualizer (`name`, `default`) |
| `audioVisualizer` on `/api/config` | `enabled` (default false), `device` (`default` or an output name), `spectrumBands` (1–32, default 16), `style` (`bands`, `wave`, `dots`, `radial`), `color` (a seek id, or empty to follow the live seek), `mirror`, `peakFall` (0–1, 0 hides caps, default 0.35), `updateHz` (stored, not the tick), `smoothing` (0–1, release only), `bassHz` / `midHz` / `trebleHz` as `[low, high]`. Frames are 60 Hz. Saving it emits `event: audioVisualizer` with `{ "enabled", "style", "color", "spectrumBands", "smoothing", "mirror", "peakFall" }` on `/api/events` |
| `POST /api/config/import` | merge a JB1 or JB2 JSON dump |
| `/api/seed/*` `/cmd/jb/seed/*` | seed playlists. See [Seed](seed.md) |

Twitch login, connect, and chat routes are listed in [Twitch](twitch.md).

OBS overlay: `http://localhost:42420/overlay/`. Overlay Stage: `http://localhost:42420/stage/`. Admin dock: `http://localhost:42420/admin/`.

`/overlay`, `/stage`, and `/admin` without a trailing slash 301 to the slash form so relative assets resolve.
