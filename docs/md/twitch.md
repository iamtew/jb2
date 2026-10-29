# Twitch

Log in with Twitch OAuth as the **bot** account. Join channel and bot nick are separate so the bot can sit in someone else's chat. Access tokens expire in a few hours. JB2 stores a refresh token and renews them before connecting.

Leave channel and token empty to skip IRC. If channel and token are in `jb2.sqlite` at process start, IRC auto-connects. Disconnect lasts until Connect or the next process start.

## Bot login

1. Have a Twitch account for the bot. Log the browser out of the streamer account first.
2. Admin → Twitch: **Login with Twitch**. Approve as the **bot**. Bot nick is filled from that account. Join channel defaults to the same login if you left it empty. IRC connects after a successful login.
3. **Connect** / **Disconnect** from Admin if you need to drop or rejoin without logging in again.

The Twitch app is built in (PKCE, S256). Redirect is always `http://localhost:<port>/api/twitch/callback` (default `42420`). Register that URL on the app at [dev.twitch.tv/console/apps](https://dev.twitch.tv/console/apps) if you own the Client ID. A leftover `twitchClientSecret` in `jb2.sqlite` is sent on token exchange if present (Confidential apps).

In Admin → Twitch, or in `jb2.sqlite`:

- `twitchChannel` — channel to JOIN (without `#`)
- `twitchNick` — bot login (set by OAuth; defaults to the channel name if empty)
- `twitchRefreshToken` / `twitchOAuth` — filled by Login with Twitch (do not paste by hand)

Connect and disconnect from Admin without restarting JB2. Admin → Chat is the live IRC window (send as the bot nick; nicks follow Twitch `display-name` casing). Admin → Twitch is login and connect only.

## Reconnect

Retries back off: 10s after a network drop, 30s after a token refresh failure or a repeated IRC login failure. If Twitch rejects the refresh token (revoked app), JB2 clears the stored login, stops reconnecting, and shows `Twitch login expired; log in again`. Use **Login with Twitch**.

## Chat commands

`!jb` / `!jukeboks` replies `@nick Jukeboks, request a song!` plus the command list.

`!songrequest` / `!sr`, `!queue` / `!q`, `!songinfo` / `!si`.

They run on a serial FIFO (cap 32, drop newest) so a busy chat cannot stall IRC. Admin → Commands sets per-command GCD and UCD seconds (default 0, counted from accept) and a global on/off (`twitchCommandsEnabled`, default on). Off still logs chat. HTTP `/cmd/jb/songrequest` and Admin Add/Now stay live. Accepted commands this process (with YouTube and Spotify link cards) sit under the cooldown table until restart.

What `!sr` accepts, and how the queue is rewritten, is in [Requests](requests.md).

Streamer.bot:

- `GET /cmd/jb/commands`
- `GET /cmd/jb/commands/on`
- `GET /cmd/jb/commands/off`

## HTTP

`GET /api/twitch`, `POST /api/twitch/connect`, `POST /api/twitch/disconnect`, `GET|POST /api/twitch/chat`, `GET /api/twitch/history`, `POST /api/twitch/oauth/start`, `GET /api/twitch/callback`.

The rest of the surface is in [HTTP](http.md).
