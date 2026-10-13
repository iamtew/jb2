# JB2 — Jukeboks 2.0

Rust rewrite of Jukeboks. Local HTTP control plane for [YouTube Music Desktop (pear-desktop)](https://github.com/pear-devs/pear-desktop) with Twitch chat commands, an OBS overlay, and a small admin dock.

No database. No app auth. One process, one `jb2.json`, one `webroot/`.

**Default base URL:** `http://localhost:42042` (binds `0.0.0.0:42042`, falls back through +9 if busy)

YTMD companion API: `http://localhost:26538` (open local API / authStrategy NONE). Overlay WebSocket talks to YTMD directly.

## Run

```
just run
just build
just test
just package
```

Flags: `-port`, `-bind`, `-ytmd_host`, `-ytmd_port`, `-webroot`, `-config`.

Config is created next to the exe (or CWD) as `jb2.json` if missing.

## Twitch

Paste an IRC oauth token for the **bot** account. Join channel and bot nick are separate so the bot can sit in someone else's chat.

In Admin → Twitch, or in `jb2.json`:

- `twitchChannel` — channel to JOIN (without `#`)
- `twitchNick` — bot login; defaults to the channel name if empty
- `twitchOAuth` — `oauth:` token for that bot account

Connect/disconnect from Admin without restarting JB2. Chat on that tab is the live IRC log; send posts as the bot nick.

If channel+token are in `jb2.json` at process start, IRC auto-connects. Disconnect lasts until Connect or the next process start.

Commands: `!songrequest` / `!sr`, `!queue` / `!q`, `!songinfo` / `!si`

Leave channel/token empty to skip IRC.

HTTP: `GET /api/twitch`, `POST /api/twitch/connect`, `POST /api/twitch/disconnect`, `GET|POST /api/twitch/chat`

## HTTP

Responses: `{ "exitCode": 0|1, "message"?, "data"? }`. App failures are HTTP 200 + `exitCode: 1`. YTMD hard fail is 502.

| Path | Role |
|---|---|
| `GET /health` | liveness |
| `GET /cmd/jb/songinfo` | now playing |
| `GET /cmd/jb/queueinfo` | upcoming queue (fails if paused) |
| `GET /cmd/jb/songrequest?input=` | search/queue (`now=1` plays immediately) |
| `GET /cmd/jb/search?input=` | search hits |
| `GET /cmd/jb/play` `pause` `next` `previous` | transport aliases |
| `/cmd/ytmd/*` | YTMD proxy (method suffix or `?_method=`) |
| `GET /api/queue` | admin queue rows |
| `/api/config` | blacklist / maxDuration / twitch fields |
| `/api/seed/*` `/cmd/jb/seed/*` | seed playlists |

OBS overlay: `http://localhost:42042/overlay/`
Admin dock: `http://localhost:42042/admin/`

## Logo

`webroot/img/jukeboks.png` is the app logo (favicon, landing, tray/exe icon after `just icon`).
