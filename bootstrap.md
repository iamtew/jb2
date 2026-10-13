# JB2 - Jukeboks 2.0
Twitch song request system on top of YouTube Music Desktop .
JB2 - Jukeboks 2.0 - this project.
JB, JB1 - Jukeboks, original project: C:\Users\Mikael\src\jukeboks
YTMD - YouTube Music Desktop player: https://github.com/pear-devs/pear-desktop

## What JB2 Does
Runs on Windows next to YTMD and controls it via the YTMD REST API and WebSocket available on `localhost:26538` 
Runs a local webserver listening on `0.0.0.0:42042`
Exposes an OBS overlay endpoint with "now playing" details for use in streams.
Has an admin UI available on its web server, designed to work with small viewports, for example OBS docked view.
Admin UI shows player controls, queue, and other info, just like JB1.
Exposes endpoint which can be accessed with GET commands to perform various actions: play, pause, next, previous, songrequest, queueinfo, songinfo, etc.
Systray icon with small menu to open the various endpoints and to restart and quit JB2.
Connect to Twitch Chat with an assigned account, accepts commands from Twitch chat: !songrequest, !sr (alias), !queue, !q (alias), !songinfo, !si (alias)
Logo file `jukeboks.png` to be moved to appropriate location, but also be used as icon for EXE and Systray and Favicon.
Same looking OBS overlay like JB1, with the Better VCR font.
JB2 Admin UI modern and sleek using Segoe UI font instead, compooared to old JB1.

## About Project
Successor to JB1, replacing it.
Written in Rust.
Runs little local web server.
Command runner: Just, using Justfile.

