# Requests

Chat `!sr`, Streamer.bot `GET /cmd/jb/songrequest`, and Admin Add/Now all queue through JB2 into [pear-desktop](https://github.com/pear-devs/pear-desktop). `now=1` plays the hit immediately instead of appending it.

`GET /cmd/jb/search?input=` returns search hits. `browse=` lists album tracks, artist top songs, or playlist tracks.

## What gets queued

`!sr` replies `@nick Added to queue: Artist - Title. #N in queue ~m:ss until play.` `#1` is the current track.

Text must match every significant word in the YouTube Music title and artist, or it is refused.

Spotify links resolve via oEmbed, then the same strict search. A miss is `sound wasn't found on YouTube Music`. The command-log card still shows the Spotify title.

YouTube watch URLs queue that video ID only if pear-desktop search returns it as a song. Otherwise chat gets `bad video: not playable on YouTube Music`. The command-log card still shows the YouTube title and channel (oEmbed) so Admin can judge a deny.

Accepted commands tint cyan. Denied ones tint orange.

## Policy

Blacklist entries are matched case-insensitively against the request text (title, artist, and related fields). A substring hit blocks the request: `request blocked by blacklist entry "…"`. The default entry is `Rick Astley`.

`maxDuration` defaults to 600 seconds (cap 86400). A longer track is refused. If a duration is required and pear-desktop does not report one, the request is blocked because the duration could not be verified.

Chat commands also sit behind the global switch and per-command GCD/UCD. See [Twitch](twitch.md). Those cooldowns do not apply to HTTP songrequest or Admin Add/Now.

## Queue order

A successful player response is not trusted on its own. JB2 reads the queue back and corrects pear-desktop until it matches.

While a seed playlist is active, requests insert after the current track, then after earlier requests. Leftover seed stays behind them. See [Seed](seed.md).
