# Agent notes

Single-developer project. Work directly on `master`; do not create feature branches or pull requests unless explicitly asked.

## Shipping

When the user says **ship it** (or equivalent):

1. Commit any pending relevant changes on `master`.
2. Push `master` to `origin`.

Do not open a PR, create a side branch, or ask whether to use a PR for a normal ship.

## Queue

JB2 is the source of truth and owns queue order. YTMD is a slave: after every queue write, read the queue back and correct it until it matches JB2. Do not trust insert position, move, or delete just because the HTTP call succeeded. One queue edit at a time.

## Overlay layout

Overlay Stage owns overlay position, canvas size, look, and panel size (`overlayLayout`). OBS `/overlay/` **must** follow every Stage layout change live (`event: overlay`): position, canvas, theme (including glass themes), seek, opacities, size, pulse, glass, hide, viz boxes. Same session, no refresh. Persisted layout is the reconnect fallback, not the live path. Do not treat a local CSS default as the source of truth.
