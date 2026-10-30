# Admin

Open `http://localhost:42420/admin/`.

The dock stays up while you switch pages. Playback state still comes from the [pear-desktop](https://github.com/pear-devs/pear-desktop) WebSocket. JB2 pushes its own changes over `GET /api/events` (Server-Sent Events): `twitch`, `chat`, `seed`, `commands`, `cmdHistory`, `queue`, `overlay`. Hidden tabs close that stream. Opening or focusing a page does a one-shot GET; leftover poll GETs are omitted from the request log (`METHOD path status` plus a timestamp; see [Running](running.md)).

The dock pill **Chat cmds ON/OFF** toggles the global chat-command switch (`twitchCommandsEnabled`). Off still logs chat. HTTP `/cmd/jb/songrequest` and Admin Add/Now stay live.

## Player

Previous, back 10 seconds, play, pause, forward 10 seconds, next, and shuffle. The seek bar scrubs the current track. The folded dock keeps previous, play/pause, and next.

**Add** queues a search. **Now** plays it immediately (`now=1` on the song-request call). The field suggests YouTube Music hits, including album, artist, and playlist browses. Matching rules are in [Requests](requests.md).

## Queue

The Queue page lists what JB2 thinks is coming up. **Previous** expands tracks already played. Drag a row by its handle to reorder. **Clear** asks, then deletes the pear-desktop queue.

While a seed playlist is active, song requests are inserted after the current track (then after earlier requests) instead of at the tail. JB2 owns that order and corrects pear-desktop until the queue matches. See [Seed](seed.md) and [Requests](requests.md).

## Chat

Admin → Chat is the live IRC window. Messages send as the bot nick. Nicks follow Twitch `display-name` casing. Login and connect live on the Twitch page, not here.

## Commands

Admin → Commands lists the chat commands this process accepts, with YouTube and Spotify link cards, until restart. Accepted commands tint cyan. Denied ones tint orange.

Per-command GCD and UCD are seconds, default 0, counted from accept, capped at 86400. Save writes them to `jb2.sqlite`. The global on/off switch is the same one as the dock pill.

## Twitch

Join channel and bot nick are separate, so the bot can sit in someone else's chat. **Login with Twitch** runs the built-in PKCE flow. **Connect** and **Disconnect** drop or rejoin without logging in again. Details, including token refresh, are in [Twitch](twitch.md).

## Settings

The theme picker is stored in the browser (`localStorage` key `jb2-admin-theme`), not in `jb2.sqlite`. Choices: Default, Amber, Crimson, Forest, Violet, Teal, Rose, Steel, Lime, each with a high-contrast variant.

`maxDuration` defaults to 600 seconds. Requests longer than that are refused. See [Requests](requests.md).

The blacklist is a list of artist or title substrings, case-insensitive. The default entry is `Rick Astley`. A hit on the title, artist, or other checked text blocks the request.

Import a JB1 `jukeboks.json` or a JB2 `jb2.json`. Present fields overwrite current settings. Empty and `***` secrets are skipped, so a JB1 dump cannot wipe Twitch. JB1 files do not touch Twitch. The page also shows the current settings JSON.

The bottom of the page runs a built-in command with optional query params (for example `seconds=10`) and shows the URL and the JSON body. Copy the URL for Streamer.bot or curl. The route list is in [HTTP](http.md).

## About

Product page: name, logo, the build version from `GET /health`, a short description, and links to GitHub, releases, docs, and credits. MIT, copyright, and pear-desktop are listed there too. The Windows tray **About** item still shows the version string.
