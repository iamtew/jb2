use crate::config::{self, Config, SeedPlaylist, Store};
use crate::envelope::Envelope;
use crate::events::EventBus;
use crate::jsonutil::{mime_for, parse_query, percent_decode, query_get, split_url};
use crate::policy;
use crate::seed::State;
use crate::twitch;
use crate::videoid::{self, MAX_SONG_REQUEST_INPUT_LEN};
use crate::ytmd::{self, Client, SongLookup};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::RecvTimeoutError;
use std::sync::Arc;
use std::time::Duration;
use tiny_http::{Header, Method, Request, Response, StatusCode};

/// Largest request body read; config imports and chat posts are a few KB.
const MAX_BODY_BYTES: u64 = 1 << 20;
const BAD_VIDEO_MSG: &str = "bad video: not playable on YouTube Music";

pub struct App {
    pub store: Store,
    pub ytmd: Client,
    pub webroot: PathBuf,
    pub seed: State,
    pub twitch: twitch::Hub,
    pub events: EventBus,
}

impl App {
    pub fn new(store: Store, ytmd: Client, webroot: PathBuf) -> Arc<Self> {
        let events = EventBus::new();
        Arc::new(Self {
            store,
            ytmd,
            webroot,
            seed: State::new(),
            twitch: twitch::Hub::with_events(events.clone()),
            events,
        })
    }

    pub fn handle(&self, mut request: Request) {
        let method = request.method().clone();
        let url = request.url().to_string();
        let (path_raw, query_raw) = split_url(&url);
        let path = percent_decode(path_raw);
        let query = parse_query(query_raw);
        let host = header_host(&request);
        let method_s = method_str(&method);
        if !is_quiet_poll_get(&method_s, &path) {
            eprintln!("{method_s} {path}");
        }

        // No auth on purpose (LAN use from OBS / Streamer.bot), so block the one thing a
        // random web page can do: fire a cross-site write. Non-browser clients send no Origin.
        if !matches!(method, Method::Get | Method::Head) {
            if let Some(origin) = header_value(&request, "Origin") {
                if !origin_allowed(&origin, &host) {
                    eprintln!("blocked cross-origin {method_s} {path} from {origin}");
                    respond_json(
                        request,
                        403,
                        &Envelope::fail("cross-origin request blocked"),
                    );
                    return;
                }
            }
        }
        if request
            .body_length()
            .is_some_and(|n| n as u64 > MAX_BODY_BYTES)
        {
            respond_json(request, 413, &Envelope::fail("request body too large"));
            return;
        }
        let mut body = Vec::new();
        let _ = request
            .as_reader()
            .take(MAX_BODY_BYTES)
            .read_to_end(&mut body);

        if method_s == "GET" && path.trim_end_matches('/') == "/api/events" {
            self.serve_events(request);
            return;
        }

        if path.starts_with("/cmd/ytmd") {
            self.proxy_ytmd(request, &method_s, &path, query_raw, &query, &body);
            return;
        }

        let env = self.dispatch(&method_s, &path, &query, &body, &host);
        match env {
            Dispatch::Json(status, envelope) => respond_json(request, status, &envelope),
            Dispatch::Html(status, body) => respond_html(request, status, &body),
            Dispatch::Static => self.serve_static(request, &path, query_raw),
        }
    }

    fn serve_events(&self, request: Request) {
        let Some(rx) = self.events.subscribe() else {
            respond_json(request, 503, &Envelope::fail("too many event listeners"));
            return;
        };
        let mut writer = request.into_writer();
        let ok = writer
            .write_all(
                b"HTTP/1.1 200 OK\r\n\
Content-Type: text/event-stream\r\n\
Cache-Control: no-cache\r\n\
Connection: keep-alive\r\n\
Transfer-Encoding: chunked\r\n\
\r\n",
            )
            .and_then(|_| sse_chunk(&mut writer, b": connected\n\n"));
        if ok.is_err() {
            return;
        }
        loop {
            match rx.recv_timeout(Duration::from_secs(15)) {
                Ok(frame) => {
                    if sse_chunk(&mut writer, frame.as_bytes()).is_err() {
                        break;
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    if sse_chunk(&mut writer, b": ping\n\n").is_err() {
                        break;
                    }
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    fn emit(&self, kind: &str, data: Value) {
        self.events.emit(kind, &data);
    }

    fn emit_queue(&self) {
        self.emit("queue", json!({}));
    }

    fn emit_seed(&self) {
        self.emit("seed", json!({}));
    }

    fn emit_commands_enabled(&self, enabled: bool) {
        self.emit("commands", json!({"enabled": enabled}));
    }

    fn emit_twitch_status(&self, cfg: &Config) {
        self.emit("twitch", self.twitch.status_json(cfg));
    }

    fn dispatch(
        &self,
        method: &str,
        path: &str,
        query: &[(String, String)],
        body: &[u8],
        _host: &str,
    ) -> Dispatch {
        let p = path.trim_end_matches('/');
        match (method, p) {
            ("GET", "/health") => Dispatch::Json(
                200,
                Envelope::ok_msg(
                    "ok",
                    json!({"features": ["seed"], "version": crate::version()}),
                ),
            ),
            ("GET", "/api/config") => self.config_get(),
            ("POST" | "PUT", "/api/config") => self.config_save(body),
            ("POST", "/api/config/import") => self.config_import(body),
            ("GET", "/api/twitch") => self.twitch_status(),
            ("POST", "/api/twitch/connect") => self.twitch_connect(),
            ("POST", "/api/twitch/disconnect") => self.twitch_disconnect(),
            ("GET", "/api/twitch/chat") => self.twitch_chat_get(),
            ("GET", "/api/twitch/history") => self.twitch_history_get(),
            ("POST", "/api/twitch/chat") => self.twitch_chat_send(body),
            ("POST", "/api/twitch/oauth/start") => self.twitch_oauth_start(),
            ("GET", "/api/twitch/callback") => self.twitch_oauth_callback(query),
            ("GET", "/api/queue") => self.admin_queue(),
            ("GET", "/api/seed/status") => self.seed_status(),
            ("POST", "/api/seed/resolve") => self.seed_resolve(query, body),
            ("POST", "/api/seed/playlists") => self.seed_add_playlist(query, body),
            ("POST", "/api/seed/settings") => self.seed_settings(body),
            ("GET", "/cmd/jb/commands") => self.chat_commands_status(),
            ("GET" | "POST", "/cmd/jb/commands/on") => self.chat_commands_set(true),
            ("GET" | "POST", "/cmd/jb/commands/off") => self.chat_commands_set(false),
            ("GET", "/cmd/jb/songinfo") => self.song_info(),
            ("GET", "/cmd/jb/queueinfo") => self.queue_info(),
            ("GET", "/cmd/jb/search") => self.search(query),
            ("GET", "/cmd/jb/songrequest") => self.song_request(
                query_get(query, "input").unwrap_or(""),
                song_request_play_now(query),
            ),
            ("GET" | "POST", "/cmd/jb/play") => self.alias_ytmd("POST", "/api/v1/play"),
            ("GET" | "POST", "/cmd/jb/pause") => self.alias_ytmd("POST", "/api/v1/pause"),
            ("GET" | "POST", "/cmd/jb/next") => self.alias_ytmd("POST", "/api/v1/next"),
            ("GET" | "POST", "/cmd/jb/previous") => self.alias_ytmd("POST", "/api/v1/previous"),
            ("POST", "/cmd/jb/seed/enqueue") => self.seed_enqueue(query),
            ("DELETE", "/cmd/jb/seed/queue") => self.seed_remove_from_queue(query),
            (_, p) if p.starts_with("/cmd/jb/") => Dispatch::Json(
                404,
                Envelope::fail(format!("unknown jukeboks command: {method} {path}")),
            ),
            ("DELETE", p) if p.starts_with("/api/seed/playlists/") => {
                let id = p.trim_start_matches("/api/seed/playlists/");
                self.seed_delete_playlist(id)
            }
            (_, p) if p.starts_with("/api/") => Dispatch::Json(
                404,
                Envelope::fail(format!(
                    "unknown API route: {path} (restart jb2 if this endpoint should exist)"
                )),
            ),
            _ => Dispatch::Static,
        }
    }

    pub fn song_info(&self) -> Dispatch {
        match self
            .ytmd
            .fetch_json_retry("/api/v1/song", 3, Duration::from_millis(250))
        {
            Ok(payload) => Dispatch::Json(200, ytmd::build_song_info_response(&payload)),
            Err(e) => Dispatch::Json(
                200,
                Envelope::fail(format!("failed to reach song endpoint: {e}")),
            ),
        }
    }

    pub fn queue_info(&self) -> Dispatch {
        let song = match self
            .ytmd
            .fetch_json_retry("/api/v1/song", 3, Duration::from_millis(250))
        {
            Ok(v) => v,
            Err(e) => {
                return Dispatch::Json(
                    200,
                    Envelope::fail(format!("failed to reach song endpoint: {e}")),
                )
            }
        };
        let queue = match self
            .ytmd
            .fetch_json_retry("/api/v1/queue", 3, Duration::from_millis(250))
        {
            Ok(v) => v,
            Err(e) => {
                return Dispatch::Json(
                    200,
                    Envelope::fail(format!("failed to reach queue endpoint: {e}")),
                )
            }
        };
        Dispatch::Json(200, ytmd::build_queue_info_response(&song, &queue))
    }

    fn search(&self, query: &[(String, String)]) -> Dispatch {
        let input = query_get(query, "input").unwrap_or("").trim();
        if input.is_empty() {
            return Dispatch::Json(200, Envelope::fail("missing input parameter"));
        }
        if input.len() > MAX_SONG_REQUEST_INPUT_LEN {
            return Dispatch::Json(200, Envelope::fail("input too long"));
        }
        match ytmd::search_songs(&self.ytmd, input, 8) {
            Ok(items) => Dispatch::Json(
                200,
                Envelope::ok_msg(
                    format!("{} search results", items.len()),
                    json!({"items": items}),
                ),
            ),
            Err(e) => Dispatch::Json(200, Envelope::fail(format!("failed to search: {e}"))),
        }
    }

    pub fn song_request(&self, input: &str, play_now: bool) -> Dispatch {
        let input = input.trim();
        if input.is_empty() {
            return Dispatch::Json(200, Envelope::fail("missing input parameter"));
        }
        if input.len() > MAX_SONG_REQUEST_INPUT_LEN {
            return Dispatch::Json(200, Envelope::fail("input too long"));
        }

        let mut link_thumb = String::new();
        let mut link_title = String::new();
        let mut link_artist = String::new();
        let mut youtube_url = false;
        let (video_id, lookup) = if let Some(track_url) =
            videoid::canonical_spotify_track_url(input)
        {
            match ytmd::lookup_spotify_track_query(&track_url) {
                Ok(sp) => {
                    link_thumb = sp.thumbnail;
                    link_title = sp.title;
                    link_artist = sp.artist;
                    match ytmd::lookup_song_by_query(&self.ytmd, &sp.query) {
                        Ok(v) => v,
                        Err(_) => {
                            let msg = if link_title.is_empty() {
                                "sound wasn't found on YouTube Music".to_string()
                            } else if link_artist.is_empty() {
                                format!("sound wasn't found on YouTube Music: {link_title}")
                            } else {
                                format!(
                                    "sound wasn't found on YouTube Music: {link_title} — {link_artist}"
                                )
                            };
                            return fail_link(&msg, &link_thumb, &link_title, &link_artist);
                        }
                    }
                }
                Err(e) => {
                    return fail_link(
                        &format!("failed to resolve Spotify track: {e}"),
                        "",
                        "",
                        "",
                    )
                }
            }
        } else {
            let (video_id, url_like) = videoid::classify_song_request_input(input);
            if !video_id.is_empty() {
                youtube_url = url_like;
                if url_like {
                    link_thumb = format!("https://i.ytimg.com/vi/{video_id}/mqdefault.jpg");
                }
                if let Some((title, channel, thumb)) = ytmd::lookup_youtube_oembed(&video_id) {
                    link_title = title;
                    link_artist = channel;
                    if !thumb.is_empty() {
                        link_thumb = thumb;
                    }
                }
                match ytmd::lookup_song_by_video_id(&self.ytmd, &video_id) {
                    Some(lookup) => (video_id, lookup),
                    None => {
                        return fail_link(
                            BAD_VIDEO_MSG,
                            &link_thumb,
                            &link_title,
                            &link_artist,
                        );
                    }
                }
            } else if url_like {
                return Dispatch::Json(200, Envelope::fail("unsupported or invalid YouTube URL"));
            } else {
                match ytmd::lookup_song_by_query(&self.ytmd, input) {
                    Ok(v) => v,
                    Err(e) => return Dispatch::Json(200, Envelope::fail(e)),
                }
            }
        };

        let cfg = match self.store.get() {
            Ok(c) => c,
            Err(e) => {
                return Dispatch::Json(200, Envelope::fail(format!("failed to load config: {e}")))
            }
        };
        // ponytail: YouTube URL with no YTMD search hit has no duration; YTMD accept/refuse is the check.
        let require_duration = lookup.duration > 0 || !youtube_url;
        if let Err(e) = policy::check_content(
            &cfg,
            lookup.duration,
            require_duration,
            &[input, &lookup.title, &lookup.artist],
        ) {
            return fail_link(&e, &link_thumb, &link_title, &link_artist);
        }

        let mut queue_payload =
            match self
                .ytmd
                .fetch_json_retry("/api/v1/queue", 3, Duration::from_millis(250))
            {
                Ok(v) => v,
                Err(e) => {
                    return Dispatch::Json(
                        200,
                        Envelope::fail(format!("failed to reach queue endpoint: {e}")),
                    )
                }
            };
        let song_payload = self
            .ytmd
            .fetch_json_retry("/api/v1/song", 3, Duration::from_millis(250))
            .unwrap_or(Value::Null);
        let current_video_id = ytmd::current_song_video_id(&song_payload);

        if play_now {
            return match self.play_song_now(&video_id, &current_video_id, queue_payload, &lookup) {
                Ok(env) => {
                    self.emit_queue();
                    self.emit_seed();
                    Dispatch::Json(
                        200,
                        attach_link_preview(env, &link_thumb, &link_title, &link_artist),
                    )
                }
                Err(e) => {
                    if youtube_url {
                        fail_link(BAD_VIDEO_MSG, &link_thumb, &link_title, &link_artist)
                    } else {
                        Dispatch::Json(200, Envelope::fail(e))
                    }
                }
            };
        }

        if ytmd::queue_contains_video_id(&queue_payload, &video_id)
            || current_video_id.eq_ignore_ascii_case(&video_id)
        {
            return Dispatch::Json(
                200,
                attach_link_preview(
                    duplicate_song(&video_id, &lookup),
                    &link_thumb,
                    &link_title,
                    &link_artist,
                ),
            );
        }

        if self.seed.is_active() {
            if cfg.clear_queue_on_request {
                if let Err(e) = self.remove_upcoming_seed_tracks(
                    &queue_payload,
                    &self.seed.video_id_set(),
                    &current_video_id,
                ) {
                    return Dispatch::Json(
                        200,
                        Envelope::fail(format!("failed to clear seed queue: {e}")),
                    );
                }
                queue_payload =
                    match self
                        .ytmd
                        .fetch_json_retry("/api/v1/queue", 3, Duration::from_millis(250))
                    {
                        Ok(v) => v,
                        Err(e) => {
                            return Dispatch::Json(
                                200,
                                Envelope::fail(format!("failed to reach queue endpoint: {e}")),
                            )
                        }
                    };
            }
            match self.insert_request_during_seed(&queue_payload, &song_payload, &video_id) {
                Ok(q) => queue_payload = q,
                Err(e) => {
                    if youtube_url {
                        return fail_link(
                            BAD_VIDEO_MSG,
                            &link_thumb,
                            &link_title,
                            &link_artist,
                        );
                    }
                    return Dispatch::Json(
                        200,
                        Envelope::fail(format!("failed to reach queue endpoint: {e}")),
                    );
                }
            }
        } else if let Err(e) = self.ytmd.post_json(
            "/api/v1/queue",
            json!({"videoId": video_id, "insertPosition": "INSERT_AT_END"}),
        ) {
            if youtube_url {
                return fail_link(BAD_VIDEO_MSG, &link_thumb, &link_title, &link_artist);
            }
            return Dispatch::Json(
                200,
                Envelope::fail(format!("failed to reach queue endpoint: {e}")),
            );
        }

        if youtube_url {
            match self.wait_for_queue_video(&video_id, 12, Duration::from_millis(150)) {
                Ok((q, _)) => queue_payload = q,
                Err(_) => {
                    return fail_link(BAD_VIDEO_MSG, &link_thumb, &link_title, &link_artist);
                }
            }
        } else if let Ok(fresh) =
            self.ytmd
                .fetch_json_retry("/api/v1/queue", 3, Duration::from_millis(250))
        {
            queue_payload = fresh;
        }
        self.seed.sync_from_queue(&queue_payload, &current_video_id);
        let mut env = ytmd::build_song_request_response(&video_id, &queue_payload, &lookup);
        if play_now {
            if let Some(msg) = env.message.as_mut() {
                *msg = msg.replacen("Added to queue:", "Playing now:", 1);
            }
        }
        self.emit_queue();
        self.emit_seed();
        Dispatch::Json(
            200,
            attach_link_preview(env, &link_thumb, &link_title, &link_artist),
        )
    }

    fn play_song_now(
        &self,
        video_id: &str,
        current_video_id: &str,
        mut queue_payload: Value,
        lookup: &SongLookup,
    ) -> Result<Envelope, String> {
        if current_video_id.eq_ignore_ascii_case(video_id) {
            return Ok(playing_now(ytmd::build_song_request_response(
                video_id,
                &queue_payload,
                lookup,
            )));
        }
        let idx = ytmd::queue_index_for_video_id(&queue_payload, video_id, true);
        if idx >= 0 {
            self.patch_queue_index(idx)?;
            if let Ok(fresh) =
                self.ytmd
                    .fetch_json_retry("/api/v1/queue", 3, Duration::from_millis(250))
            {
                queue_payload = fresh;
            }
            self.seed.sync_from_queue(&queue_payload, video_id);
            return Ok(playing_now(ytmd::build_song_request_response(
                video_id,
                &queue_payload,
                lookup,
            )));
        }
        self.ytmd.post_json(
            "/api/v1/queue",
            json!({"videoId": video_id, "insertPosition": "INSERT_AFTER_CURRENT_VIDEO"}),
        )?;
        let (mut fresh, index) =
            self.wait_for_queue_video(video_id, 12, Duration::from_millis(150))?;
        self.patch_queue_index(index)?;
        if let Ok(q) = self
            .ytmd
            .fetch_json_retry("/api/v1/queue", 3, Duration::from_millis(250))
        {
            fresh = q;
        }
        self.seed.sync_from_queue(&fresh, video_id);
        Ok(playing_now(ytmd::build_song_request_response(
            video_id, &fresh, lookup,
        )))
    }

    fn config_get(&self) -> Dispatch {
        match self.store.get() {
            Ok(cfg) => Dispatch::Json(200, Envelope::ok_data(config::public_json(&cfg))),
            Err(e) => Dispatch::Json(200, Envelope::fail(format!("failed to load config: {e}"))),
        }
    }

    fn config_save(&self, body: &[u8]) -> Dispatch {
        let parsed: Value = match serde_json::from_slice(body) {
            Ok(v) => v,
            Err(e) => {
                return Dispatch::Json(200, Envelope::fail(format!("failed to decode config: {e}")))
            }
        };
        let saved = match self.store.update(|cfg| {
            config::apply_patch(cfg, &parsed);
            Ok(())
        }) {
            Ok(c) => c,
            Err(e) => {
                return Dispatch::Json(200, Envelope::fail(format!("failed to save config: {e}")))
            }
        };
        if config::touches_twitch(&parsed) {
            self.twitch.bump_if_wanted();
            self.emit_twitch_status(&saved);
        }
        if !saved.twitch_commands_enabled {
            self.twitch.clear_jobs();
        }
        self.emit_commands_enabled(saved.twitch_commands_enabled);
        if parsed.get("clearQueueOnRequest").is_some() || parsed.get("seedPlaylists").is_some() {
            self.emit_seed();
        }
        Dispatch::Json(
            200,
            Envelope::ok_msg("config saved", config::public_json(&saved)),
        )
    }

    fn config_import(&self, body: &[u8]) -> Dispatch {
        let parsed: Value = match serde_json::from_slice(body) {
            Ok(v) => v,
            Err(e) => {
                return Dispatch::Json(200, Envelope::fail(format!("failed to decode import: {e}")))
            }
        };
        let source = match config::detect_import(&parsed) {
            Ok(s) => s,
            Err(e) => return Dispatch::Json(200, Envelope::fail(e)),
        };
        let saved = match self.store.update(|cfg| {
            config::apply_patch(cfg, &parsed);
            Ok(())
        }) {
            Ok(c) => c,
            Err(e) => {
                return Dispatch::Json(200, Envelope::fail(format!("failed to save import: {e}")))
            }
        };
        if config::touches_twitch(&parsed) {
            self.twitch.bump_if_wanted();
            self.emit_twitch_status(&saved);
        }
        if !saved.twitch_commands_enabled {
            self.twitch.clear_jobs();
        }
        self.emit_commands_enabled(saved.twitch_commands_enabled);
        self.emit_seed();
        let mut data = config::public_json(&saved);
        if let Some(obj) = data.as_object_mut() {
            obj.insert("source".into(), json!(source.as_str()));
        }
        Dispatch::Json(
            200,
            Envelope::ok_msg(format!("imported {}", source.as_str()), data),
        )
    }

    fn chat_commands_status(&self) -> Dispatch {
        match self.store.get() {
            Ok(cfg) => Dispatch::Json(
                200,
                Envelope::ok_data(json!({"enabled": cfg.twitch_commands_enabled})),
            ),
            Err(e) => Dispatch::Json(200, Envelope::fail(format!("failed to load config: {e}"))),
        }
    }

    fn chat_commands_set(&self, enabled: bool) -> Dispatch {
        match self.store.update(|c| {
            c.twitch_commands_enabled = enabled;
            Ok(())
        }) {
            Ok(cfg) => {
                if !cfg.twitch_commands_enabled {
                    self.twitch.clear_jobs();
                }
                self.emit_commands_enabled(cfg.twitch_commands_enabled);
                Dispatch::Json(
                    200,
                    Envelope::ok_data(json!({"enabled": cfg.twitch_commands_enabled})),
                )
            }
            Err(e) => Dispatch::Json(200, Envelope::fail(format!("failed to save config: {e}"))),
        }
    }

    fn twitch_status(&self) -> Dispatch {
        match self.store.get() {
            Ok(cfg) => Dispatch::Json(200, Envelope::ok_data(self.twitch.status_json(&cfg))),
            Err(e) => Dispatch::Json(200, Envelope::fail(format!("failed to load config: {e}"))),
        }
    }

    fn twitch_connect(&self) -> Dispatch {
        if let Err(e) = twitch::ensure_access_token(&self.store, false) {
            return Dispatch::Json(200, Envelope::fail(e));
        }
        match self.store.get() {
            Ok(cfg) if cfg.twitch_channel.trim().is_empty() => {
                Dispatch::Json(200, Envelope::fail("save a join channel before connecting"))
            }
            Ok(cfg) if cfg.twitch_oauth.trim().is_empty() => {
                Dispatch::Json(200, Envelope::fail("log in with Twitch before connecting"))
            }
            Err(e) => Dispatch::Json(200, Envelope::fail(format!("failed to load config: {e}"))),
            Ok(_) => {
                self.twitch.connect();
                self.twitch_status()
            }
        }
    }

    fn twitch_oauth_start(&self) -> Dispatch {
        let redirect = twitch_redirect_uri();
        match self.twitch.begin_oauth(&redirect) {
            Ok(url) => Dispatch::Json(
                200,
                Envelope::ok_msg("authorize", json!({"url": url, "redirectUri": redirect})),
            ),
            Err(e) => Dispatch::Json(200, Envelope::fail(e)),
        }
    }

    fn twitch_oauth_callback(&self, query: &[(String, String)]) -> Dispatch {
        if let Some(err) = query_get(query, "error") {
            let desc = query_get(query, "error_description").unwrap_or(err);
            return Dispatch::Html(200, oauth_html(&format!("Twitch login failed: {desc}")));
        }
        let Some(code) = query_get(query, "code").filter(|s| !s.trim().is_empty()) else {
            return Dispatch::Html(400, oauth_html("Twitch login missing code."));
        };
        let Some(state) = query_get(query, "state") else {
            return Dispatch::Html(400, oauth_html("Twitch login missing state."));
        };
        let (redirect, verifier) = match self.twitch.take_oauth_redirect(state) {
            Ok(pair) => pair,
            Err(e) => return Dispatch::Html(400, oauth_html(&e)),
        };
        match twitch::exchange_code(&self.store, code, &redirect, &verifier) {
            Ok(login) => {
                self.twitch.connect();
                Dispatch::Html(
                    200,
                    oauth_html(&format!(
                        "JB2 logged in as {login}. You can close this tab."
                    )),
                )
            }
            Err(e) => Dispatch::Html(200, oauth_html(&format!("Twitch login failed: {e}"))),
        }
    }

    fn twitch_disconnect(&self) -> Dispatch {
        self.twitch.disconnect();
        self.twitch_status()
    }

    fn twitch_chat_get(&self) -> Dispatch {
        Dispatch::Json(200, Envelope::ok_data(self.twitch.chat_json()))
    }

    fn twitch_history_get(&self) -> Dispatch {
        Dispatch::Json(200, Envelope::ok_data(self.twitch.history_json()))
    }

    fn twitch_chat_send(&self, body: &[u8]) -> Dispatch {
        let parsed: Value = match serde_json::from_slice(body) {
            Ok(v) => v,
            Err(e) => {
                return Dispatch::Json(200, Envelope::fail(format!("failed to decode chat: {e}")))
            }
        };
        let Some(message) = parsed.get("message").and_then(|v| v.as_str()) else {
            return Dispatch::Json(200, Envelope::fail("missing message"));
        };
        match self.twitch.enqueue_send(message) {
            Ok(()) => Dispatch::Json(200, Envelope::ok_only("queued")),
            Err(e) => Dispatch::Json(200, Envelope::fail(e)),
        }
    }

    fn admin_queue(&self) -> Dispatch {
        let song = self
            .ytmd
            .fetch_json_retry("/api/v1/song", 2, Duration::from_millis(100))
            .unwrap_or(Value::Null);
        match self
            .ytmd
            .fetch_json_retry("/api/v1/queue", 2, Duration::from_millis(100))
        {
            Ok(queue) => {
                let (items, status) = ytmd::normalize_admin_queue(&song, &queue);
                let exit = if status == "error" { 1 } else { 0 };
                Dispatch::Json(
                    200,
                    Envelope {
                        exit_code: exit,
                        message: None,
                        data: Some(json!({"items": items, "status": status})),
                    },
                )
            }
            Err(e) => Dispatch::Json(
                200,
                Envelope::fail_data(
                    format!("failed to reach queue endpoint: {e}"),
                    json!({"status": "unavailable"}),
                ),
            ),
        }
    }

    fn seed_status(&self) -> Dispatch {
        let mut cfg = match self.store.get() {
            Ok(c) => c,
            Err(e) => {
                return Dispatch::Json(200, Envelope::fail(format!("failed to load config: {e}")))
            }
        };
        let fresh = self.refresh_seed_playlist_names(&cfg);
        if !fresh.is_empty() {
            // Replace only; a playlist deleted during the YTMD lookups stays deleted.
            let saved = self.store.update(|c| {
                for p in &fresh {
                    if let Some(existing) = c
                        .seed_playlists
                        .iter_mut()
                        .find(|e| e.id.eq_ignore_ascii_case(&p.id))
                    {
                        *existing = p.clone();
                    }
                }
                Ok(())
            });
            if let Ok(saved) = saved {
                cfg = saved;
            }
        }
        let (active, video_count, playlist_count) = self.seed.snapshot();
        let playing = self
            .ytmd
            .fetch_json_retry("/api/v1/song", 2, Duration::from_millis(100))
            .map(|p| {
                self.seed
                    .classify_video_id(&ytmd::current_song_video_id(&p))
            })
            .unwrap_or("none");
        Dispatch::Json(
            200,
            Envelope::ok_data(json!({
                "seedModeActive": active,
                "activeSeedVideoCount": video_count,
                "activeSeedPlaylistCount": playlist_count,
                "activeRequestCount": self.seed.request_count(),
                "playingKind": playing,
                "clearQueueOnRequest": cfg.clear_queue_on_request,
                "seedPlaylists": cfg.seed_playlists,
            })),
        )
    }

    fn seed_resolve(&self, query: &[(String, String)], body: &[u8]) -> Dispatch {
        let input = match seed_input(query, body) {
            Ok(v) => v,
            Err(e) => return Dispatch::Json(200, Envelope::fail(e)),
        };
        let playlist_id = match videoid::extract_playlist_id(&input) {
            Ok(v) => v,
            Err(e) => return Dispatch::Json(200, Envelope::fail(e)),
        };
        match ytmd::lookup_playlist_by_id(&self.ytmd, &playlist_id) {
            Ok(lookup) => Dispatch::Json(
                200,
                Envelope::ok_msg(
                    format!("Resolved playlist: {}", lookup.name),
                    json!({"id": lookup.id, "name": lookup.name, "trackCount": lookup.tracks.len()}),
                ),
            ),
            Err(e) => Dispatch::Json(
                200,
                Envelope::fail(format!("failed to resolve playlist: {e}")),
            ),
        }
    }

    fn seed_add_playlist(&self, query: &[(String, String)], body: &[u8]) -> Dispatch {
        let input = match seed_input(query, body) {
            Ok(v) => v,
            Err(e) => return Dispatch::Json(200, Envelope::fail(e)),
        };
        let playlist_id = match videoid::extract_playlist_id(&input) {
            Ok(v) => v,
            Err(e) => return Dispatch::Json(200, Envelope::fail(e)),
        };
        let lookup = match ytmd::lookup_playlist_by_id(&self.ytmd, &playlist_id) {
            Ok(v) => v,
            Err(e) => {
                return Dispatch::Json(
                    200,
                    Envelope::fail(format!("failed to resolve playlist: {e}")),
                )
            }
        };
        let playlist = SeedPlaylist {
            id: lookup.id.clone(),
            name: lookup.name.clone(),
            track_count: lookup.tracks.len() as i32,
        };
        if let Err(e) = self.update_seed_playlist(&playlist) {
            return Dispatch::Json(200, Envelope::fail(format!("failed to save playlist: {e}")));
        }
        self.emit_seed();
        Dispatch::Json(
            200,
            Envelope::ok_msg(
                format!("Saved seed playlist: {}", lookup.name),
                serde_json::to_value(&playlist).unwrap_or(json!({})),
            ),
        )
    }

    fn seed_delete_playlist(&self, playlist_id: &str) -> Dispatch {
        let playlist_id = playlist_id.trim();
        if playlist_id.is_empty() {
            return Dispatch::Json(200, Envelope::fail("missing playlist id"));
        }
        if let Err(e) = self.remove_seed_playlist(playlist_id) {
            return Dispatch::Json(
                200,
                Envelope::fail(format!("failed to remove playlist: {e}")),
            );
        }
        self.emit_seed();
        Dispatch::Json(200, Envelope::ok_only("Seed playlist removed"))
    }

    fn seed_settings(&self, body: &[u8]) -> Dispatch {
        let parsed: Value = match serde_json::from_slice(body) {
            Ok(v) => v,
            Err(e) => {
                return Dispatch::Json(
                    200,
                    Envelope::fail(format!("failed to decode settings: {e}")),
                )
            }
        };
        let Some(flag) = parsed.get("clearQueueOnRequest").and_then(|v| v.as_bool()) else {
            return Dispatch::Json(200, Envelope::fail("missing clearQueueOnRequest"));
        };
        let cfg = match self.store.update(|c| {
            c.clear_queue_on_request = flag;
            Ok(())
        }) {
            Ok(c) => c,
            Err(e) => {
                return Dispatch::Json(200, Envelope::fail(format!("failed to save settings: {e}")))
            }
        };
        self.emit_seed();
        Dispatch::Json(
            200,
            Envelope::ok_msg(
                "Seed settings saved",
                json!({"clearQueueOnRequest": cfg.clear_queue_on_request}),
            ),
        )
    }

    fn seed_enqueue(&self, query: &[(String, String)]) -> Dispatch {
        let playlist_id = query_get(query, "playlistId").unwrap_or("").trim();
        if playlist_id.is_empty() {
            return Dispatch::Json(200, Envelope::fail("missing playlistId parameter"));
        }
        if self.find_saved_seed(playlist_id).is_none() {
            return Dispatch::Json(
                200,
                Envelope::fail("playlist is not in saved seed playlists"),
            );
        }
        let cfg = match self.store.get() {
            Ok(c) => c,
            Err(e) => {
                return Dispatch::Json(200, Envelope::fail(format!("failed to load config: {e}")))
            }
        };
        let lookup = match ytmd::lookup_playlist_by_id(&self.ytmd, playlist_id) {
            Ok(v) => v,
            Err(e) => {
                return Dispatch::Json(
                    200,
                    Envelope::fail(format!("failed to look up playlist: {e}")),
                )
            }
        };
        let mut before_ids = HashSet::new();
        if let Ok(queue) =
            self.ytmd
                .fetch_json_retry("/api/v1/queue", 2, Duration::from_millis(250))
        {
            for id in ytmd::queue_item_video_ids(&queue) {
                before_ids.insert(id.trim().to_lowercase());
            }
        }
        let (added, skipped, video_ids) = match self.enqueue_seed_tracks(&cfg, &lookup) {
            Ok(v) => v,
            Err(e) => {
                return Dispatch::Json(
                    200,
                    Envelope::fail(format!("failed to enqueue seed playlist: {e}")),
                )
            }
        };
        if added == 0 {
            return Dispatch::Json(200, Envelope::fail("no tracks were added to the queue"));
        }
        self.seed.register_enqueue(&lookup.id, &video_ids);
        if let Ok(queue) =
            self.ytmd
                .fetch_json_retry("/api/v1/queue", 3, Duration::from_millis(250))
        {
            let live: Vec<String> = ytmd::queue_item_video_ids(&queue)
                .into_iter()
                .map(|id| id.trim().to_lowercase())
                .filter(|id| !id.is_empty() && !before_ids.contains(id))
                .collect();
            if !live.is_empty() {
                self.seed.register_enqueue(&lookup.id, &live);
            }
        }
        let _ = self.update_seed_playlist(&SeedPlaylist {
            id: lookup.id.clone(),
            name: lookup.name.clone(),
            track_count: lookup.tracks.len() as i32,
        });
        self.emit_queue();
        self.emit_seed();
        Dispatch::Json(
            200,
            Envelope::ok_msg(
                format!("Added {added} tracks from {} to queue", lookup.name),
                json!({
                    "playlistId": lookup.id,
                    "name": lookup.name,
                    "added": added,
                    "skipped": skipped,
                    "trackCount": lookup.tracks.len(),
                    "seedModeActive": true
                }),
            ),
        )
    }

    fn seed_remove_from_queue(&self, query: &[(String, String)]) -> Dispatch {
        let playlist_id = query_get(query, "playlistId").unwrap_or("").trim();
        if playlist_id.is_empty() {
            return Dispatch::Json(200, Envelope::fail("missing playlistId parameter"));
        }
        let lookup = match ytmd::lookup_playlist_by_id(&self.ytmd, playlist_id) {
            Ok(v) => v,
            Err(e) => {
                return Dispatch::Json(
                    200,
                    Envelope::fail(format!("failed to look up playlist: {e}")),
                )
            }
        };
        let target: HashSet<String> = lookup
            .tracks
            .iter()
            .map(|t| t.video_id.trim().to_lowercase())
            .collect();
        let queue = match self
            .ytmd
            .fetch_json_retry("/api/v1/queue", 3, Duration::from_millis(250))
        {
            Ok(v) => v,
            Err(e) => {
                return Dispatch::Json(
                    200,
                    Envelope::fail(format!("failed to reach queue endpoint: {e}")),
                )
            }
        };
        let indices = ytmd::queue_indices_for_video_ids(&queue, &target);
        if let Err(e) = self.delete_queue_indices(&indices) {
            return Dispatch::Json(
                200,
                Envelope::fail(format!("failed to remove seed tracks from queue: {e}")),
            );
        }
        let queue = self
            .ytmd
            .fetch_json_retry("/api/v1/queue", 2, Duration::from_millis(250))
            .unwrap_or(Value::Null);
        let song = self
            .ytmd
            .fetch_json_retry("/api/v1/song", 2, Duration::from_millis(250))
            .unwrap_or(Value::Null);
        self.seed
            .sync_from_queue(&queue, &ytmd::current_song_video_id(&song));
        self.emit_queue();
        self.emit_seed();
        Dispatch::Json(
            200,
            Envelope::ok_msg(
                format!("Removed {} seed tracks from queue", indices.len()),
                json!({"removed": indices.len(), "seedModeActive": self.seed.is_active()}),
            ),
        )
    }

    fn alias_ytmd(&self, method: &str, endpoint: &str) -> Dispatch {
        match self.ytmd.request_json(method, endpoint, None) {
            Ok(data) => Dispatch::Json(200, Envelope::ok_data(data)),
            Err(e) => Dispatch::Json(502, Envelope::fail(format!("failed to reach YTMD: {e}"))),
        }
    }

    fn proxy_ytmd(
        &self,
        request: Request,
        method: &str,
        path: &str,
        query_raw: &str,
        query: &[(String, String)],
        body: &[u8],
    ) {
        let cfg = match self.store.get() {
            Ok(c) => c,
            Err(e) => {
                respond_json(
                    request,
                    200,
                    &Envelope::fail(format!("failed to load config: {e}")),
                );
                return;
            }
        };
        if let Err(e) = policy::enforce(&cfg, query, body) {
            respond_json(request, 200, &Envelope::fail(e));
            return;
        }
        let mut method = ytmd::resolve_upstream_method(method, path);
        if method.eq_ignore_ascii_case("GET") {
            if let Some(m) = query_get(query, "_method") {
                if !m.is_empty() {
                    method = m.to_ascii_uppercase();
                }
            }
        }
        let upstream_path = ytmd::route_for_path(path);
        let body_bytes = ytmd::build_upstream_body(&method, query, body);
        match self
            .ytmd
            .proxy(&method, &upstream_path, query_raw, body_bytes)
        {
            Ok((status, bytes)) if status < 400 => {
                let decoded = if bytes.iter().all(|b| b.is_ascii_whitespace()) {
                    Envelope {
                        exit_code: 0,
                        message: Some("ok".into()),
                        data: None,
                    }
                } else if let Ok(v) = serde_json::from_slice::<Value>(&bytes) {
                    Envelope::ok_data(v)
                } else {
                    Envelope::ok_data(Value::String(String::from_utf8_lossy(&bytes).into_owned()))
                };
                if queue_mutating(&method, &upstream_path) {
                    self.emit_queue();
                }
                respond_json(request, 200, &decoded);
            }
            Ok((status, bytes)) => {
                respond_json(
                    request,
                    502,
                    &Envelope::fail(format!(
                        "YTMD returned {status}: {}",
                        String::from_utf8_lossy(&bytes).trim()
                    )),
                );
            }
            Err(e) => {
                respond_json(
                    request,
                    502,
                    &Envelope::fail(format!("failed to reach YTMD at {}: {e}", self.ytmd.base)),
                );
            }
        }
    }

    fn serve_static(&self, request: Request, path: &str, query_raw: &str) {
        let rel = path.trim_start_matches('/');
        let mut candidate = if rel.is_empty() {
            self.webroot.join("index.html")
        } else {
            self.webroot.join(rel)
        };
        // /overlay without a slash resolves relative CSS/JS against `/`, not `/overlay/`.
        if candidate.is_dir() {
            if let Some(loc) = dir_slash_location(path, query_raw) {
                respond_redirect(request, 301, &loc);
                return;
            }
            candidate = candidate.join("index.html");
        }
        if path == "/favicon.ico" {
            let ico = self.webroot.join("img/jukeboks.ico");
            let png = self.webroot.join("img/jukeboks.png");
            candidate = if ico.exists() { ico } else { png };
        }
        if !is_inside(&self.webroot, &candidate) {
            let _ = request
                .respond(Response::from_string("not found").with_status_code(StatusCode(404)));
            return;
        }
        match fs::read(&candidate) {
            Ok(bytes) => {
                let mime = mime_for(candidate.to_str().unwrap_or(""));
                let mut resp = Response::from_data(bytes);
                if let Ok(h) = Header::from_bytes(&b"Content-Type"[..], mime.as_bytes()) {
                    resp = resp.with_header(h);
                }
                let _ = request.respond(resp);
            }
            Err(_) => {
                let _ = request
                    .respond(Response::from_string("not found").with_status_code(StatusCode(404)));
            }
        }
    }

    fn patch_queue_index(&self, index: i32) -> Result<(), String> {
        self.ytmd
            .request_json("PATCH", "/api/v1/queue", Some(json!({"index": index})))
            .map(|_| ())
    }

    fn delete_queue_index(&self, index: i32) -> Result<(), String> {
        self.ytmd
            .request_json("DELETE", &format!("/api/v1/queue/{index}"), None)
            .map(|_| ())
            .map_err(|e| format!("failed to delete queue item {index}: {e}"))
    }

    fn delete_queue_indices(&self, indices: &[i32]) -> Result<(), String> {
        let mut sorted = indices.to_vec();
        sorted.sort_unstable_by(|a, b| b.cmp(a));
        for index in sorted {
            self.delete_queue_index(index)?;
        }
        Ok(())
    }

    fn move_queue_item(&self, from_index: i32, to_index: i32) -> Result<(), String> {
        if from_index == to_index {
            return Ok(());
        }
        self.ytmd
            .request_json(
                "PATCH",
                &format!("/api/v1/queue/{from_index}"),
                Some(json!({"toIndex": to_index})),
            )
            .map(|_| ())
            .map_err(|e| format!("failed to move queue item {from_index} to {to_index}: {e}"))
    }

    fn wait_for_queue_video(
        &self,
        video_id: &str,
        attempts: usize,
        delay: Duration,
    ) -> Result<(Value, i32), String> {
        for attempt in 0..attempts {
            let queue = self.ytmd.fetch_json_retry("/api/v1/queue", 2, delay)?;
            let index = ytmd::queue_index_for_video_id(&queue, video_id, true);
            if index >= 0 {
                return Ok((queue, index));
            }
            if attempt + 1 < attempts {
                std::thread::sleep(delay);
            }
        }
        Err(format!(
            "queued video \"{video_id}\" not visible in YTMD queue yet"
        ))
    }

    fn remove_upcoming_seed_tracks(
        &self,
        queue_payload: &Value,
        seed_video_ids: &HashSet<String>,
        current_video_id: &str,
    ) -> Result<(), String> {
        let current_index = ytmd::resolve_current_queue_index(queue_payload, current_video_id);
        let indices = ytmd::queue_indices_for_video_ids(queue_payload, seed_video_ids);
        let to_delete: Vec<i32> = indices
            .into_iter()
            .filter(|index| !(current_index >= 0 && *index <= current_index))
            .collect();
        self.delete_queue_indices(&to_delete)
    }

    fn repair_seed_request_block(
        &self,
        mut queue_payload: Value,
        current_video_id: &str,
    ) -> Result<Value, String> {
        let requests = self.seed.request_ids();
        if requests.is_empty() {
            return Ok(queue_payload);
        }
        let mut current_index = ytmd::resolve_current_queue_index(&queue_payload, current_video_id);
        if current_index < 0 {
            return Ok(queue_payload);
        }
        let mut offset = 0i32;
        for video_id in &requests {
            let fresh =
                self.ytmd
                    .fetch_json_retry("/api/v1/queue", 3, Duration::from_millis(150))?;
            queue_payload = fresh;
            current_index = ytmd::resolve_current_queue_index(&queue_payload, current_video_id);
            if current_index < 0 {
                return Ok(queue_payload);
            }
            let actual = ytmd::queue_index_for_video_id(&queue_payload, video_id, false);
            if actual >= 0 && actual <= current_index {
                continue;
            }
            let desired = current_index + 1 + offset;
            offset += 1;
            if actual < 0 || actual == desired {
                continue;
            }
            self.move_queue_item(actual, desired)?;
            std::thread::sleep(Duration::from_millis(150));
        }
        Ok(self
            .ytmd
            .fetch_json_retry("/api/v1/queue", 3, Duration::from_millis(150))
            .unwrap_or(queue_payload))
    }

    fn insert_request_during_seed(
        &self,
        queue_payload: &Value,
        song_payload: &Value,
        request_video_id: &str,
    ) -> Result<Value, String> {
        let current_video_id = ytmd::current_song_video_id(song_payload);
        self.seed.sync_from_queue(queue_payload, &current_video_id);
        let queue_payload =
            self.repair_seed_request_block(queue_payload.clone(), &current_video_id)?;
        self.seed.sync_from_queue(&queue_payload, &current_video_id);
        let mut target_index = self
            .seed
            .request_insert_index(&queue_payload, &current_video_id);
        self.ytmd.post_json(
            "/api/v1/queue",
            json!({"videoId": request_video_id, "insertPosition": "INSERT_AFTER_CURRENT_VIDEO"}),
        )?;
        let (mut fresh_queue, mut from_index) =
            self.wait_for_queue_video(request_video_id, 12, Duration::from_millis(150))?;
        let current_index = ytmd::resolve_current_queue_index(&fresh_queue, &current_video_id);
        if target_index < 0 {
            target_index = self
                .seed
                .request_insert_index(&fresh_queue, &current_video_id);
        }
        if target_index < 0 && current_index >= 0 {
            target_index = current_index + 1;
        }
        if target_index >= 0 && from_index != target_index {
            if let Err(e) = self.move_queue_item(from_index, target_index) {
                self.seed.append_request(request_video_id);
                return Err(e);
            }
            std::thread::sleep(Duration::from_millis(150));
            if let Ok((q, idx)) =
                self.wait_for_queue_video(request_video_id, 8, Duration::from_millis(150))
            {
                fresh_queue = q;
                from_index = idx;
                if idx != target_index {
                    let _ = self.move_queue_item(from_index, target_index);
                    if let Ok(q2) =
                        self.ytmd
                            .fetch_json_retry("/api/v1/queue", 3, Duration::from_millis(150))
                    {
                        fresh_queue = q2;
                    }
                }
            }
        }
        self.seed.append_request(request_video_id);
        self.seed.sync_from_queue(&fresh_queue, &current_video_id);
        Ok(fresh_queue)
    }

    fn enqueue_seed_tracks(
        &self,
        cfg: &Config,
        lookup: &ytmd::PlaylistLookup,
    ) -> Result<(i32, i32, Vec<String>), String> {
        let queue_payload = self
            .ytmd
            .fetch_json_retry("/api/v1/queue", 2, Duration::from_millis(250))
            .ok();
        let queue_was_empty = queue_payload
            .as_ref()
            .map(|q| ytmd::queue_item_video_ids(q).is_empty())
            .unwrap_or(true);
        let mut added = 0;
        let mut skipped = 0;
        let mut video_ids = Vec::new();
        for track in &lookup.tracks {
            let mut candidates = vec![
                track.video_id.as_str(),
                track.title.as_str(),
                track.artist.as_str(),
            ];
            let combo;
            if !track.title.is_empty() && !track.artist.is_empty() {
                combo = format!("{} {}", track.title, track.artist);
                candidates.push(combo.as_str());
            }
            if policy::check_content(cfg, track.duration, track.duration > 0, &candidates).is_err()
            {
                skipped += 1;
                continue;
            }
            if let Some(q) = &queue_payload {
                if ytmd::queue_contains_video_id(q, &track.video_id) {
                    skipped += 1;
                    continue;
                }
            }
            self.ytmd.post_json(
                "/api/v1/queue",
                json!({"videoId": track.video_id, "insertPosition": "INSERT_AT_END"}),
            )?;
            added += 1;
            video_ids.push(track.video_id.clone());
        }
        if queue_was_empty && added > 0 {
            let _ = self.ytmd.post_json("/api/v1/play", json!({}));
        }
        Ok((added, skipped, video_ids))
    }

    fn update_seed_playlist(&self, playlist: &SeedPlaylist) -> Result<(), String> {
        self.store
            .update(|cfg| {
                if let Some(existing) = cfg
                    .seed_playlists
                    .iter_mut()
                    .find(|p| p.id.eq_ignore_ascii_case(&playlist.id))
                {
                    *existing = playlist.clone();
                } else {
                    cfg.seed_playlists.push(playlist.clone());
                }
                Ok(())
            })
            .map(|_| ())
    }

    fn remove_seed_playlist(&self, playlist_id: &str) -> Result<(), String> {
        self.store
            .update(|cfg| {
                cfg.seed_playlists
                    .retain(|p| !p.id.eq_ignore_ascii_case(playlist_id));
                Ok(())
            })
            .map(|_| ())
    }

    fn find_saved_seed(&self, playlist_id: &str) -> Option<SeedPlaylist> {
        self.store
            .get()
            .ok()?
            .seed_playlists
            .into_iter()
            .find(|p| p.id.eq_ignore_ascii_case(playlist_id))
    }

    /// Look up real names for placeholder-named playlists. Network only; returns the
    /// refreshed entries for the caller to merge via `Store::update`.
    fn refresh_seed_playlist_names(&self, cfg: &Config) -> Vec<SeedPlaylist> {
        let mut fresh = Vec::new();
        for playlist in &cfg.seed_playlists {
            let name = playlist.name.trim();
            let id = playlist.id.trim();
            if !(name.is_empty() || name.eq_ignore_ascii_case(id) || name.starts_with("map[")) {
                continue;
            }
            if let Ok(lookup) = ytmd::lookup_playlist_by_id(&self.ytmd, &playlist.id) {
                if lookup.name.trim().is_empty()
                    || lookup.name.trim().eq_ignore_ascii_case(&playlist.id)
                {
                    continue;
                }
                let mut p = playlist.clone();
                p.name = lookup.name;
                if !lookup.tracks.is_empty() {
                    p.track_count = lookup.tracks.len() as i32;
                }
                fresh.push(p);
            }
        }
        fresh
    }
}

pub enum Dispatch {
    Json(u16, Envelope),
    Html(u16, String),
    Static,
}

impl Dispatch {
    pub fn envelope(self) -> Envelope {
        match self {
            Dispatch::Json(_, e) => e,
            Dispatch::Html(_, msg) => Envelope::ok_only(msg),
            Dispatch::Static => Envelope::fail("not a json route"),
        }
    }
}

fn song_request_play_now(query: &[(String, String)]) -> bool {
    let now = query_get(query, "now").unwrap_or("").trim();
    if now == "1" || now.eq_ignore_ascii_case("true") {
        return true;
    }
    query_get(query, "play")
        .unwrap_or("")
        .trim()
        .eq_ignore_ascii_case("now")
}

fn seed_input(query: &[(String, String)], body: &[u8]) -> Result<String, String> {
    if let Some(v) = query_get(query, "input") {
        if !v.trim().is_empty() {
            return Ok(v.trim().to_string());
        }
    }
    if let Ok(v) = serde_json::from_slice::<Value>(body) {
        if let Some(s) = v.get("input").and_then(|x| x.as_str()) {
            if !s.trim().is_empty() {
                return Ok(s.trim().to_string());
            }
        }
    }
    Err("missing input".into())
}

fn fail_link(message: &str, thumb: &str, link_title: &str, link_artist: &str) -> Dispatch {
    Dispatch::Json(
        200,
        attach_link_preview(
            Envelope::fail_data(message, json!({})),
            thumb,
            link_title,
            link_artist,
        ),
    )
}

fn attach_link_preview(
    mut env: Envelope,
    thumb: &str,
    link_title: &str,
    link_artist: &str,
) -> Envelope {
    if thumb.is_empty() && link_title.is_empty() && link_artist.is_empty() {
        return env;
    }
    let mut data = env.data.take().unwrap_or(json!({}));
    if let Some(obj) = data.as_object_mut() {
        if !thumb.is_empty() {
            obj.insert("thumbnail".into(), json!(thumb));
        }
        if !link_title.is_empty() {
            obj.insert("linkTitle".into(), json!(link_title));
        }
        if !link_artist.is_empty() {
            obj.insert("linkArtist".into(), json!(link_artist));
        }
    }
    env.data = Some(data);
    env
}

fn duplicate_song(video_id: &str, lookup: &SongLookup) -> Envelope {
    let message = if !lookup.title.is_empty() && !lookup.artist.is_empty() {
        format!(
            "Song already in queue: {} — {}.",
            lookup.title, lookup.artist
        )
    } else if !lookup.title.is_empty() {
        format!("Song already in queue: {}.", lookup.title)
    } else {
        "Song already in queue.".into()
    };
    let mut data = json!({"reason": "duplicate", "videoId": video_id});
    if !lookup.title.is_empty() {
        data["title"] = json!(lookup.title);
    }
    if !lookup.artist.is_empty() {
        data["artist"] = json!(lookup.artist);
    }
    Envelope::fail_data(message, data)
}

fn playing_now(mut env: Envelope) -> Envelope {
    if let Some(msg) = env.message.as_mut() {
        *msg = msg.replacen("Added to queue:", "Playing now:", 1);
    }
    env
}

fn sse_chunk(writer: &mut dyn Write, data: &[u8]) -> io::Result<()> {
    write!(writer, "{:x}\r\n", data.len())?;
    writer.write_all(data)?;
    writer.write_all(b"\r\n")?;
    writer.flush()
}

fn queue_mutating(method: &str, path: &str) -> bool {
    !method.eq_ignore_ascii_case("GET") && path.contains("queue")
}

fn is_quiet_poll_get(method: &str, path: &str) -> bool {
    method == "GET"
        && matches!(
            path,
            "/api/queue"
                | "/api/twitch"
                | "/api/twitch/chat"
                | "/api/twitch/history"
                | "/api/seed/status"
                | "/cmd/jb/commands"
                | "/cmd/ytmd/song"
                | "/cmd/ytmd/song/get"
                | "/cmd/ytmd/shuffle"
                | "/cmd/ytmd/shuffle/get"
        )
}

fn method_str(m: &Method) -> String {
    match m {
        Method::Get => "GET".into(),
        Method::Post => "POST".into(),
        Method::Put => "PUT".into(),
        Method::Delete => "DELETE".into(),
        Method::Patch => "PATCH".into(),
        Method::Head => "HEAD".into(),
        Method::Options => "OPTIONS".into(),
        other => format!("{other:?}"),
    }
}

fn header_value(request: &Request, name: &'static str) -> Option<String> {
    request
        .headers()
        .iter()
        .find(|h| h.field.equiv(name))
        .map(|h| h.value.as_str().trim().to_string())
        .filter(|s| !s.is_empty())
}

fn header_host(request: &Request) -> String {
    header_value(request, "Host").unwrap_or_else(|| "localhost:42420".into())
}

/// Same-origin check: `Origin: http://pc:42420` must name the `Host` the request was sent to.
/// `Origin: null` (file://, sandboxed frames) never matches.
fn origin_allowed(origin: &str, host: &str) -> bool {
    let origin_host = origin
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or("")
        .trim_end_matches('/');
    !origin_host.is_empty() && origin_host.eq_ignore_ascii_case(host.trim())
}

fn twitch_redirect_uri() -> String {
    format!(
        "http://localhost:{}/api/twitch/callback",
        crate::httpctl::listen_port()
    )
}

fn oauth_html(message: &str) -> String {
    let safe = message
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>JB2 Twitch</title></head><body><p>{safe}</p><script>window.close()</script></body></html>"
    )
}

fn dir_slash_location(path: &str, query_raw: &str) -> Option<String> {
    if path.is_empty() || path.ends_with('/') {
        return None;
    }
    if query_raw.is_empty() {
        Some(format!("{path}/"))
    } else {
        Some(format!("{path}/?{query_raw}"))
    }
}

fn respond_redirect(request: Request, status: u16, location: &str) {
    let mut resp = Response::empty(StatusCode(status));
    if let Ok(h) = Header::from_bytes(&b"Location"[..], location.as_bytes()) {
        resp = resp.with_header(h);
    }
    let _ = request.respond(resp);
}

fn respond_html(request: Request, status: u16, body: &str) {
    let mut resp = Response::from_string(body.to_string()).with_status_code(StatusCode(status));
    if let Ok(h) = Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]) {
        resp = resp.with_header(h);
    }
    let _ = request.respond(resp);
}

fn respond_json(request: Request, status: u16, envelope: &Envelope) {
    let body = serde_json::to_vec(envelope).unwrap_or_else(|_| b"{}".to_vec());
    let mut resp = Response::from_data(body).with_status_code(StatusCode(status));
    if let Ok(h) = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]) {
        resp = resp.with_header(h);
    }
    let _ = request.respond(resp);
}

fn is_inside(root: &Path, path: &Path) -> bool {
    let Ok(root) = fs::canonicalize(root) else {
        return false;
    };
    let Ok(path) = fs::canonicalize(path) else {
        return false;
    };
    path.starts_with(root)
}

#[cfg(test)]
mod tests {
    use super::{dir_slash_location, is_quiet_poll_get, origin_allowed};

    #[test]
    fn admin_poll_gets_are_quiet() {
        assert!(is_quiet_poll_get("GET", "/api/queue"));
        assert!(is_quiet_poll_get("GET", "/api/twitch"));
        assert!(is_quiet_poll_get("GET", "/api/twitch/chat"));
        assert!(is_quiet_poll_get("GET", "/api/twitch/history"));
        assert!(is_quiet_poll_get("GET", "/api/seed/status"));
        assert!(is_quiet_poll_get("GET", "/cmd/jb/commands"));
        assert!(!is_quiet_poll_get("GET", "/cmd/jb/commands/on"));
        assert!(is_quiet_poll_get("GET", "/cmd/ytmd/song/get"));
        assert!(is_quiet_poll_get("GET", "/cmd/ytmd/shuffle/get"));
        assert!(!is_quiet_poll_get("POST", "/api/twitch/chat"));
        assert!(!is_quiet_poll_get("GET", "/cmd/jb/songrequest"));
    }

    #[test]
    fn origin_must_match_host() {
        assert!(origin_allowed("http://localhost:42420", "localhost:42420"));
        assert!(origin_allowed("http://STREAM-PC:42420", "stream-pc:42420"));
        assert!(!origin_allowed("https://evil.example", "localhost:42420"));
        assert!(!origin_allowed("http://localhost:42421", "localhost:42420"));
        assert!(!origin_allowed("null", "localhost:42420"));
    }

    #[test]
    fn dir_urls_gain_a_slash() {
        assert_eq!(dir_slash_location("/overlay", ""), Some("/overlay/".into()));
        assert_eq!(
            dir_slash_location("/overlay", "dev=true"),
            Some("/overlay/?dev=true".into())
        );
        assert_eq!(dir_slash_location("/overlay/", ""), None);
        assert_eq!(dir_slash_location("/overlay/", "dev=true"), None);
        assert_eq!(dir_slash_location("/admin", ""), Some("/admin/".into()));
    }
}
