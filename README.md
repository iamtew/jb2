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

Log in with Twitch OAuth as the **bot** account. Join channel and bot nick are separate so the bot can sit in someone else's chat. Access tokens expire in a few hours; JB2 stores a refresh token and renews them before connecting.

### Bot login

1. Have a Twitch account for the bot. Log the browser out of the streamer account first.
2. Register a **Confidential** app at [dev.twitch.tv/console/apps](https://dev.twitch.tv/console/apps). OAuth Redirect URL must be exactly `http://localhost:42042/api/twitch/callback` (no trailing slash; use the same host/port you open Admin with). Create a Client Secret and copy the Client ID.
3. In Admin → Twitch: join channel, Client ID, Client Secret, **Save**, then **Login with Twitch**. Approve as the **bot**. Bot nick is filled from that account.
4. **Connect**. After that, reconnects refresh the access token automatically.

If you change JB2’s port, update the Redirect URL in the Twitch console to match.

In Admin → Twitch, or in `jb2.json`:

- `twitchChannel` — channel to JOIN (without `#`)
- `twitchNick` — bot login (set by OAuth; defaults to the channel name if empty)
- `twitchClientId` / `twitchClientSecret` — your Twitch app
- `twitchRefreshToken` / `twitchOAuth` — filled by Login with Twitch (do not paste by hand)

Connect/disconnect from Admin without restarting JB2. Chat on that tab is the live IRC log; send posts as the bot nick.

If channel+token are in `jb2.json` at process start, IRC auto-connects. Disconnect lasts until Connect or the next process start.

Commands: `!songrequest` / `!sr`, `!queue` / `!q`, `!songinfo` / `!si`

Leave channel/token empty to skip IRC.

HTTP: `GET /api/twitch`, `POST /api/twitch/connect`, `POST /api/twitch/disconnect`, `GET|POST /api/twitch/chat`, `POST /api/twitch/oauth/start`, `GET /api/twitch/callback`

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
