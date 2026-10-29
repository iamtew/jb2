# Overview

JB2 is a Rust rewrite of Jukeboks. It is a local HTTP control plane for [YouTube Music Desktop (pear-desktop)](https://github.com/pear-devs/pear-desktop): Twitch chat commands, an OBS overlay, Overlay Stage, and a small admin dock.

One process, one `jb2.sqlite`, one `webroot/`.

**Default base URL:** `http://localhost:42420` (binds `0.0.0.0:42420`; exits if the port is busy)

The player is [pear-desktop](https://github.com/pear-devs/pear-desktop). JB2 does not work with the original [YouTube Music Desktop](https://github.com/th-ch/youtube-music). pear-desktop serves its local API at `http://localhost:26538` (open local API / authStrategy NONE). The overlay WebSocket connects there. See [Overlay](overlay.md).

## Surfaces

- **Admin dock** at `http://localhost:42420/admin/`. Queue, seed playlists, Twitch chat, command cooldowns, settings, and a small API runner. See [Admin](admin.md).
- **OBS overlay** at `http://localhost:42420/overlay/`. Now playing, playback state, and progress. See [Overlay](overlay.md).
- **Overlay Stage** at `http://localhost:42420/stage/`. Place the overlay on a 720p or 1080p canvas; OBS follows live. See [Overlay Stage](stage.md).
- **HTTP and chat.** Twitch `!sr` and Streamer.bot call the same song-request path the admin dock uses. See [Twitch](twitch.md), [Requests](requests.md), and [HTTP](http.md).

`/overlay`, `/stage`, and `/admin` without a trailing slash 301 to the slash form so relative assets resolve.

## Website

The markdown in this folder is the operator manual. `docs/index.html` is the product landing page. `docs/docs.html` renders these files. GitHub Pages serves the `docs/` folder as-is (branch `master`, folder `/docs`). `docs/.nojekyll` is there so Jekyll does not rewrite the markdown.
