# Seed

A seed playlist keeps [pear-desktop](https://github.com/pear-devs/pear-desktop) from running dry. Admin → Seed takes a playlist URL or ID. JB2 stores the id, name, and track count on the playlist (`seedPlaylists` in `jb2.sqlite`).

While seed mode is active, song requests insert after the current track (then after earlier requests). Leftover seed stays after those requests. JB2 owns that order and corrects pear-desktop until the queue matches. A successful player response is not trusted on its own. See [Requests](requests.md).

**Clear queue on request** (`clearQueueOnRequest`, default off) drops upcoming seed tracks when a request lands, so the request is not stuck behind the filler.

The status line reports whether seed mode is active. Removing a playlist from the list deletes it from settings.

## HTTP

- `GET /api/seed/status`
- `POST /api/seed/resolve` — resolve a playlist URL or id
- `POST /api/seed/playlists` — add one
- `POST /api/seed/settings` — body `{ "clearQueueOnRequest": true|false }`
- `DELETE /api/seed/playlists/<id>`
- `POST /cmd/jb/seed/enqueue`
- `DELETE /cmd/jb/seed/queue`

`GET /health` reports `data.features` including `seed`.
