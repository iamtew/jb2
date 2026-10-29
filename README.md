# JB2 — Jukeboks 2.0

Rust rewrite of Jukeboks. Local HTTP control plane for [YouTube Music Desktop (pear-desktop)](https://github.com/pear-devs/pear-desktop) with Twitch chat commands, an OBS overlay, and a small admin dock.

No app auth. One process, one `jb2.sqlite`, one `webroot/`.

**Default base URL:** `http://localhost:42420` (binds `0.0.0.0:42420`; exits if the port is busy)

YTMD companion API: `http://localhost:26538` (open local API / authStrategy NONE). Overlay WebSocket talks to YTMD directly.

## Run

```
just run
just build
just test
just package
```

Flags: `-port`, `-bind`, `-ytmd_host`, `-ytmd_port`, `-webroot`, `-config`.

Config is created next to the exe (or CWD) as `jb2.sqlite` if missing. If a sibling `jb2.json` exists and the database is empty, it is imported once (the JSON file is left in place).

Admin → Settings accepts old JB1 `jukeboks.json` and JB2 `jb2.json` files. Present fields overwrite current settings; empty and `***` secrets are skipped so a JB1 dump cannot wipe Twitch.

## Twitch

Log in with Twitch OAuth as the **bot** account. Join channel and bot nick are separate so the bot can sit in someone else's chat. Access tokens expire in a few hours; JB2 stores a refresh token and renews them before connecting.

### Bot login

1. Have a Twitch account for the bot. Log the browser out of the streamer account first.
2. Admin → Twitch: **Login with Twitch**. Approve as the **bot**. Bot nick is filled from that account; join channel defaults to the same login if you left it empty. IRC connects after a successful login.
3. **Connect** / **Disconnect** from Admin if you need to drop or rejoin without logging in again.

The Twitch app is built in (PKCE). Redirect is always `http://localhost:<port>/api/twitch/callback` (default `42420`). Register that URL on the app at [dev.twitch.tv/console/apps](https://dev.twitch.tv/console/apps) if you own the Client ID. A leftover `twitchClientSecret` in `jb2.sqlite` is sent on token exchange if present (Confidential apps).

In Admin → Twitch, or in `jb2.sqlite`:

- `twitchChannel` — channel to JOIN (without `#`)
- `twitchNick` — bot login (set by OAuth; defaults to the channel name if empty)
- `twitchRefreshToken` / `twitchOAuth` — filled by Login with Twitch (do not paste by hand)

Connect/disconnect from Admin without restarting JB2. Admin → Chat is the live IRC window (send as the bot nick; nicks follow Twitch `display-name` casing). Admin → Twitch is login/connect only. The dock pill **Chat cmds ON/OFF** toggles the global chat-command switch. Admin → Commands lists accepted chat commands for this process (YouTube/Spotify link cards) until restart.

If channel+token are in `jb2.sqlite` at process start, IRC auto-connects. Disconnect lasts until Connect or the next process start.

Retries back off: 10s after a network drop, 30s after a token refresh failure or a repeated IRC login failure. If Twitch rejects the refresh token (revoked app), JB2 clears the stored login, stops reconnecting, and shows `Twitch login expired; log in again` — use **Login with Twitch**.

Chat commands: `!songrequest` / `!sr`, `!queue` / `!q`, `!songinfo` / `!si`. They run on a serial FIFO (cap 32, drop newest) so a busy chat cannot stall IRC. Admin → Commands sets per-command GCD/UCD seconds (default 0, counted from accept) and a global on/off (`twitchCommandsEnabled`, default on). Off still logs chat; HTTP `/cmd/jb/songrequest` and Admin Add/Now stay live. Accepted commands this process (with YouTube/Spotify link cards) sit under the cooldown table until restart.

`!sr` text must match every significant word in the YouTube Music title+artist or it is refused. Spotify links resolve via oEmbed then the same strict search; a miss is `sound wasn't found on YouTube Music` (card still shows the Spotify title). YouTube watch URLs queue that video ID only; if YTMD will not play it, chat gets `bad video: not playable on YouTube Music`. While a jb2 seed playlist is active, requests insert after the current track (then after earlier requests); leftover seed stays.

Streamer.bot: `GET /cmd/jb/commands`, `GET /cmd/jb/commands/on`, `GET /cmd/jb/commands/off`

Leave channel/token empty to skip IRC.

HTTP: `GET /api/twitch`, `POST /api/twitch/connect`, `POST /api/twitch/disconnect`, `GET|POST /api/twitch/chat`, `GET /api/twitch/history`, `POST /api/twitch/oauth/start`, `GET /api/twitch/callback`

## HTTP

Responses: `{ "exitCode": 0|1, "message"?, "data"? }`. App failures are HTTP 200 + `exitCode: 1`. YTMD hard fail is 502. Unknown `/api/*` and `/cmd/jb/*` routes are 404.

No auth, so the LAN (OBS, Streamer.bot on another PC) can reach everything. Browser writes (POST/PUT/PATCH/DELETE) whose `Origin` does not match the `Host` they were sent to get 403, so a random web page cannot change config or chat as the bot. Clients that send no `Origin` (Streamer.bot, curl) are unaffected. Request bodies over 1 MiB get 413.

| Path | Role |
|---|---|
| `GET /health` | liveness (`data.version` is `{git tag}-{git hash short}`, plus `-dirty` if the build tree was dirty) |
| `GET /cmd/jb/songinfo` | now playing |
| `GET /cmd/jb/queueinfo` | upcoming queue (fails if paused) |
| `GET /cmd/jb/songrequest?input=` | search/queue (`now=1` plays immediately) |
| `GET /cmd/jb/search?input=` | search hits |
| `GET /cmd/jb/commands` | Twitch chat commands on/off |
| `GET /cmd/jb/commands/on` `off` | enable/disable Twitch chat commands |
| `GET /cmd/jb/play` `pause` `next` `previous` | transport aliases |
| `/cmd/ytmd/*` | YTMD proxy (method suffix or `?_method=`) |
| `GET /api/events` | Admin SSE (`twitch` / `chat` / `seed` / `commands` / `cmdHistory` / `queue`) |
| `GET /api/queue` | admin queue rows |
| `/api/config` | blacklist / maxDuration / twitch / command cooldowns |
| `POST /api/config/import` | merge JB1/JB2 JSON dump |
| `/api/seed/*` `/cmd/jb/seed/*` | seed playlists |

OBS overlay: `http://localhost:42420/overlay/`
Admin dock: `http://localhost:42420/admin/` (`GET /api/events` SSE for Twitch/chat/seed/commands/cmdHistory/queue; playback still uses the YTMD WebSocket. Hidden tabs close the stream. One-shot GETs on open/focus; leftover poll GETs are omitted from the request log)

`/overlay` and `/admin` without a trailing slash 301 to the slash form so relative assets resolve.

## Logo

`webroot/img/jukeboks.png` is the app logo (favicon, landing, tray/exe icon after `just icon`). Admin → About and the tray **About** item show the build version from `/health`.

## License

MIT. See [LICENSE](LICENSE).
