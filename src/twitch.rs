use crate::config::{Config, Store};
use crate::envelope::Envelope;
use crate::server::App;
use native_tls::TlsConnector;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::hash_map::RandomState;
use std::collections::{HashMap, VecDeque};
use std::hash::BuildHasher;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use url::Url;

const CHAT_CAP: usize = 200;
const MAX_PRIVMSG: usize = 500;
/// ponytail: one FIFO, drop newest past this; per-user queues if a raid starves others.
const MAX_CMD_JOBS: usize = 32;
const OAUTH_SCOPES: &str = "chat:read chat:edit";
/// Public Twitch app Client ID (not a secret). Console must list localhost callback URLs.
const TWITCH_CLIENT_ID: &str = "lax4082xr34poez274g2dzmabbwlg8";
const TOKEN_SKEW_SECS: i64 = 120;
/// Wait between retries after a token or repeated IRC auth failure, so a bad login
/// cannot hammer id.twitch.tv / irc.chat.twitch.tv.
const AUTH_BACKOFF_SECS: u64 = 30;
/// Wait between retries after a network / server-closed error.
const RECONNECT_SECS: u64 = 10;
pub const LOGIN_EXPIRED: &str = "Twitch login expired; log in again";

pub struct Hub {
    inner: Mutex<Inner>,
    jobs_cv: Condvar,
}

struct Inner {
    wanted: bool,
    connected: bool,
    generation: u64,
    channel: String,
    nick: String,
    last_error: String,
    next_id: u64,
    chat: VecDeque<ChatLine>,
    outbound: VecDeque<String>,
    jobs: VecDeque<CmdJob>,
    last_gcd: HashMap<String, Instant>,
    last_ucd: HashMap<(String, String), Instant>,
    /// (state, redirect_uri, pkce verifier)
    oauth_state: Option<(String, String, String)>,
}

struct CmdJob {
    id: String,
    args: String,
}

#[derive(Clone)]
struct ChatLine {
    id: u64,
    ts: u64,
    nick: String,
    text: String,
    /// Twitch IRCv3 `color` tag (`#RRGGBB`), empty if unset or invalid.
    color: String,
}

impl Hub {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                wanted: false,
                connected: false,
                generation: 0,
                channel: String::new(),
                nick: String::new(),
                last_error: String::new(),
                next_id: 1,
                chat: VecDeque::new(),
                outbound: VecDeque::new(),
                jobs: VecDeque::new(),
                last_gcd: HashMap::new(),
                last_ucd: HashMap::new(),
                oauth_state: None,
            }),
            jobs_cv: Condvar::new(),
        }
    }

    pub fn connect(&self) {
        let mut s = self.inner.lock().expect("twitch");
        s.wanted = true;
        s.last_error.clear();
        s.generation = s.generation.wrapping_add(1);
    }

    pub fn disconnect(&self) {
        let mut s = self.inner.lock().expect("twitch");
        s.wanted = false;
        s.connected = false;
        s.outbound.clear();
        s.generation = s.generation.wrapping_add(1);
    }

    /// Drop the live IRC socket so the worker re-reads config.
    pub fn bump_if_wanted(&self) {
        let mut s = self.inner.lock().expect("twitch");
        if s.wanted {
            s.generation = s.generation.wrapping_add(1);
        }
    }

    pub fn set_wanted(&self, wanted: bool) {
        self.inner.lock().expect("twitch").wanted = wanted;
    }

    fn wanted(&self) -> bool {
        self.inner.lock().expect("twitch").wanted
    }

    fn generation(&self) -> u64 {
        self.inner.lock().expect("twitch").generation
    }

    fn set_session(&self, connected: bool, channel: &str, nick: &str) {
        let mut s = self.inner.lock().expect("twitch");
        s.connected = connected;
        s.channel = channel.to_string();
        s.nick = nick.to_string();
        if connected {
            s.last_error.clear();
        }
    }

    fn set_error(&self, err: &str) {
        let mut s = self.inner.lock().expect("twitch");
        s.connected = false;
        s.last_error = err.to_string();
    }

    fn push_chat(&self, nick: &str, text: &str, color: &str) {
        let mut s = self.inner.lock().expect("twitch");
        let id = s.next_id;
        s.next_id = s.next_id.saturating_add(1);
        s.chat.push_back(ChatLine {
            id,
            ts: now_ms(),
            nick: nick.to_string(),
            text: text.to_string(),
            color: if is_hex_color(color) {
                color.to_string()
            } else {
                String::new()
            },
        });
        while s.chat.len() > CHAT_CAP {
            s.chat.pop_front();
        }
    }

    fn pop_outbound(&self) -> Option<String> {
        self.inner.lock().expect("twitch").outbound.pop_front()
    }

    fn prepend_outbound(&self, text: String) {
        self.inner.lock().expect("twitch").outbound.push_front(text);
    }

    pub fn enqueue_send(&self, text: &str) -> Result<(), String> {
        let text = sanitize_privmsg(text);
        if text.is_empty() {
            return Err("empty message".into());
        }
        let mut s = self.inner.lock().expect("twitch");
        if !s.connected {
            return Err("not connected".into());
        }
        // Same cap as cmd jobs: drop newest so a stuck IRC write cannot grow forever.
        if s.outbound.len() >= MAX_CMD_JOBS {
            return Err("outbound full".into());
        }
        s.outbound.push_back(text);
        Ok(())
    }

    pub fn try_enqueue_cmd(&self, cfg: &Config, nick: &str, id: &str, args: &str) -> bool {
        if !cfg.twitch_commands_enabled {
            return false;
        }
        let nick_key = nick.to_ascii_lowercase();
        let mut s = self.inner.lock().expect("twitch");
        if s.jobs.len() >= MAX_CMD_JOBS {
            return false;
        }
        if cooldown_blocked(&s, cfg, id, &nick_key) {
            return false;
        }
        stamp_use(&mut s, id, &nick_key);
        s.jobs.push_back(CmdJob {
            id: id.to_string(),
            args: args.to_string(),
        });
        self.jobs_cv.notify_one();
        true
    }

    pub fn clear_jobs(&self) {
        self.inner.lock().expect("twitch").jobs.clear();
    }

    fn wait_job(&self) -> CmdJob {
        let mut s = self.inner.lock().expect("twitch");
        loop {
            if let Some(job) = s.jobs.pop_front() {
                return job;
            }
            s = self.jobs_cv.wait(s).expect("twitch");
        }
    }

    #[cfg(test)]
    fn pending_jobs(&self) -> usize {
        self.inner.lock().expect("twitch").jobs.len()
    }

    #[cfg(test)]
    fn pop_job(&self) -> Option<CmdJob> {
        self.inner.lock().expect("twitch").jobs.pop_front()
    }

    pub fn begin_oauth(&self, redirect_uri: &str) -> Result<String, String> {
        let state = random_state();
        let verifier = pkce_verifier();
        let challenge = pkce_challenge(&verifier);
        let mut s = self.inner.lock().expect("twitch");
        s.oauth_state = Some((state.clone(), redirect_uri.to_string(), verifier));
        authorize_url(redirect_uri, &state, &challenge)
    }

    pub fn take_oauth_redirect(&self, state: &str) -> Result<(String, String), String> {
        let mut s = self.inner.lock().expect("twitch");
        match s.oauth_state.take() {
            Some((want, redirect, verifier)) if want == state => Ok((redirect, verifier)),
            _ => Err("oauth state mismatch; click Login with Twitch again".into()),
        }
    }

    pub fn status_json(&self, cfg: &Config) -> Value {
        let s = self.inner.lock().expect("twitch");
        json!({
            "connected": s.connected,
            "wanted": s.wanted,
            "channel": s.channel,
            "nick": s.nick,
            "lastError": s.last_error,
            "oauthSet": !cfg.twitch_oauth.trim().is_empty(),
            "refreshSet": !cfg.twitch_refresh_token.trim().is_empty(),
        })
    }

    pub fn chat_json(&self) -> Value {
        let s = self.inner.lock().expect("twitch");
        let messages: Vec<Value> = s
            .chat
            .iter()
            .map(|m| {
                json!({
                    "id": m.id,
                    "ts": m.ts,
                    "nick": m.nick,
                    "text": m.text,
                    "color": m.color,
                })
            })
            .collect();
        json!({ "messages": messages })
    }
}

pub fn spawn(app: Arc<App>) {
    if let Ok(cfg) = app.store.get() {
        if can_auto_connect(&cfg) {
            app.twitch.set_wanted(true);
        }
    }
    std::thread::Builder::new()
        .name("twitch".into())
        .spawn({
            let app = Arc::clone(&app);
            move || worker(app)
        })
        .expect("twitch thread");
    std::thread::Builder::new()
        .name("twitch-cmd".into())
        .spawn(move || cmd_worker(app))
        .expect("twitch-cmd thread");
}

fn cmd_worker(app: Arc<App>) {
    loop {
        let job = app.twitch.wait_job();
        let Ok(cfg) = app.store.get() else {
            continue;
        };
        if !cfg.twitch_commands_enabled {
            continue;
        }
        let reply = run_chat_command(&app, &job.id, &job.args);
        let reply = sanitize_privmsg(&reply);
        if !reply.is_empty() {
            let _ = app.twitch.enqueue_send(&reply);
        }
    }
}

fn run_chat_command(app: &App, id: &str, args: &str) -> String {
    match id {
        "sr" => {
            if args.is_empty() {
                "usage: !sr <song>".to_string()
            } else {
                envelope_text(app.song_request(args, false).envelope())
            }
        }
        "q" => envelope_text(app.queue_info().envelope()),
        "si" => envelope_text(app.song_info().envelope()),
        _ => String::new(),
    }
}

fn stamp_use(s: &mut Inner, id: &str, nick: &str) {
    let now = Instant::now();
    s.last_gcd.insert(id.to_string(), now);
    s.last_ucd
        .insert((id.to_string(), nick.to_ascii_lowercase()), now);
}

fn cooldown_blocked(s: &Inner, cfg: &Config, id: &str, nick: &str) -> bool {
    let Some(cd) = cfg.twitch_commands.get(id) else {
        return false;
    };
    let now = Instant::now();
    if cooldown_hit(s.last_gcd.get(id).copied(), now, cd.gcd) {
        return true;
    }
    cooldown_hit(
        s.last_ucd.get(&(id.to_string(), nick.to_string())).copied(),
        now,
        cd.ucd,
    )
}

fn cooldown_hit(last: Option<Instant>, now: Instant, secs: i32) -> bool {
    if secs <= 0 {
        return false;
    }
    last.map(|t| now.saturating_duration_since(t).as_secs() < secs as u64)
        .unwrap_or(false)
}

pub fn canonical_command(cmd: &str) -> Option<&'static str> {
    match cmd {
        "songrequest" | "sr" => Some("sr"),
        "queue" | "q" => Some("q"),
        "songinfo" | "si" => Some("si"),
        _ => None,
    }
}

fn worker(app: Arc<App>) {
    // Two IRC auth failures this close together mean a refresh did not help.
    let mut last_auth_fail: Option<Instant> = None;
    loop {
        if !app.twitch.wanted() {
            app.twitch.set_session(false, "", "");
            std::thread::sleep(Duration::from_millis(200));
            continue;
        }
        let gen = app.twitch.generation();
        if let Err(e) = ensure_access_token(&app.store, false) {
            app.twitch.set_error(&e);
            if e == LOGIN_EXPIRED {
                // Nothing to retry with; stay down until Login with Twitch + Connect.
                app.twitch.set_wanted(false);
                continue;
            }
            eprintln!("twitch: {e}; retry in {AUTH_BACKOFF_SECS}s");
            wait_or_cancel(&app, gen, AUTH_BACKOFF_SECS);
            continue;
        }
        let Some((channel, oauth, nick)) = app.store.get().ok().and_then(|c| creds_from_config(&c))
        else {
            app.twitch
                .set_error("set join channel and log in with Twitch");
            std::thread::sleep(Duration::from_millis(500));
            continue;
        };
        match run(&app, &channel, &oauth, &nick, gen) {
            Ok(()) => {}
            Err(e) => {
                let secs = if is_auth_fail(&e) {
                    // Expire the stored token so the next pass forces a refresh.
                    let _ = app.store.update(|c| {
                        c.twitch_oauth_expires_at = 0;
                        Ok(())
                    });
                    let repeated = last_auth_fail
                        .map(|t| t.elapsed() < Duration::from_secs(AUTH_BACKOFF_SECS * 2))
                        .unwrap_or(false);
                    last_auth_fail = Some(Instant::now());
                    if repeated {
                        AUTH_BACKOFF_SECS
                    } else {
                        0
                    }
                } else {
                    RECONNECT_SECS
                };
                app.twitch.set_error(&e);
                eprintln!("twitch: {e}; retry in {secs}s");
                wait_or_cancel(&app, gen, secs);
            }
        }
        app.twitch.set_session(false, &channel, &nick);
    }
}

/// Sleep up to `secs` in 200ms steps so Disconnect or a config change cuts the wait short.
fn wait_or_cancel(app: &App, gen: u64, secs: u64) {
    for _ in 0..secs * 5 {
        if !app.twitch.wanted() || app.twitch.generation() != gen {
            return;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn run(app: &Arc<App>, channel: &str, oauth: &str, nick: &str, gen: u64) -> Result<(), String> {
    let token = if oauth.starts_with("oauth:") {
        oauth.to_string()
    } else {
        format!("oauth:{oauth}")
    };
    let mut stream = connect_irc()?;
    send_irc(
        &mut stream,
        &format!(
            "PASS {token}\r\nNICK {nick}\r\nCAP REQ :twitch.tv/tags\r\nJOIN #{channel}\r\n"
        ),
    )?;
    let mut joined = false;
    let mut leftover = String::new();
    let mut tmp = [0u8; 8192];
    loop {
        if !app.twitch.wanted() || app.twitch.generation() != gen {
            return Ok(());
        }
        if joined {
            while let Some(text) = app.twitch.pop_outbound() {
                if let Err(e) = send_irc(&mut stream, &format!("PRIVMSG #{channel} :{text}\r\n")) {
                    app.twitch.prepend_outbound(text);
                    return Err(e);
                }
                app.twitch.push_chat(nick, &text, "");
            }
        }
        match stream.read(&mut tmp) {
            Ok(0) => return Err("server closed".into()),
            Ok(n) => {
                leftover.push_str(&String::from_utf8_lossy(&tmp[..n]));
                while let Some(idx) = leftover.find('\n') {
                    let mut line: String = leftover.drain(..=idx).collect();
                    if line.ends_with('\n') {
                        line.pop();
                    }
                    if line.ends_with('\r') {
                        line.pop();
                    }
                    if !joined && is_welcome(&line) {
                        joined = true;
                        app.twitch.set_session(true, channel, nick);
                        eprintln!("twitch: joined #{channel} as {nick}");
                    }
                    handle_line(app, channel, nick, &line, &mut stream)?;
                }
            }
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(e) => return Err(e.to_string()),
        }
    }
}

fn connect_irc() -> Result<native_tls::TlsStream<TcpStream>, String> {
    let addr = "irc.chat.twitch.tv:6697"
        .to_socket_addrs()
        .map_err(|e| e.to_string())?
        .next()
        .ok_or_else(|| "irc.chat.twitch.tv: no address".to_string())?;
    let tcp = TcpStream::connect_timeout(&addr, Duration::from_secs(15)).map_err(|e| e.to_string())?;
    let _ = tcp.set_nodelay(true);
    let tls = TlsConnector::new()
        .map_err(|e| e.to_string())?
        .connect("irc.chat.twitch.tv", tcp)
        .map_err(|e| e.to_string())?;
    tls.get_ref()
        .set_nonblocking(true)
        .map_err(|e| e.to_string())?;
    Ok(tls)
}

fn send_irc(stream: &mut impl Write, s: &str) -> Result<(), String> {
    stream
        .write_all(s.as_bytes())
        .map_err(|e| format!("irc write: {e}"))?;
    stream.flush().map_err(|e| format!("irc write: {e}"))
}

fn handle_line(
    app: &Arc<App>,
    _channel: &str,
    _bot_nick: &str,
    line: &str,
    stream: &mut impl Write,
) -> Result<(), String> {
    let line = line.trim();
    if line.is_empty() {
        return Ok(());
    }
    if line.starts_with("PING") {
        return send_irc(stream, &format!("{}\r\n", line.replacen("PING", "PONG", 1)));
    }
    if is_auth_fail(line) {
        return Err(line.to_string());
    }
    let Some((from, body)) = parse_privmsg(line) else {
        return Ok(());
    };
    app.twitch.push_chat(&from, &body, &privmsg_color(line));
    let Some((cmd, rest)) = parse_command(&body) else {
        return Ok(());
    };
    let Some(id) = canonical_command(&cmd) else {
        return Ok(());
    };
    let Ok(cfg) = app.store.get() else {
        return Ok(());
    };
    let _ = app.twitch.try_enqueue_cmd(&cfg, &from, id, &rest);
    Ok(())
}

fn envelope_text(env: Envelope) -> String {
    env.message.unwrap_or_else(|| {
        if env.exit_code == 0 {
            "ok".into()
        } else {
            "failed".into()
        }
    })
}

fn can_auto_connect(cfg: &Config) -> bool {
    let channel = cfg.twitch_channel.trim();
    !channel.is_empty()
        && (!cfg.twitch_oauth.trim().is_empty() || !cfg.twitch_refresh_token.trim().is_empty())
}

fn is_welcome(line: &str) -> bool {
    line.contains(" 001 ")
}

fn is_auth_fail(msg: &str) -> bool {
    let m = msg.to_ascii_lowercase();
    m.contains("login authentication failed")
        || m.contains("improperly formatted auth")
        || m.contains("login unsuccessful")
}

fn bare_token(oauth: &str) -> String {
    oauth
        .trim()
        .strip_prefix("oauth:")
        .unwrap_or(oauth.trim())
        .to_string()
}

fn authorize_url(redirect_uri: &str, state: &str, challenge: &str) -> Result<String, String> {
    let mut url = Url::parse("https://id.twitch.tv/oauth2/authorize").map_err(|e| e.to_string())?;
    url.query_pairs_mut()
        .append_pair("client_id", TWITCH_CLIENT_ID)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", OAUTH_SCOPES)
        .append_pair("state", state)
        .append_pair("force_verify", "true")
        .append_pair("code_challenge", challenge)
        .append_pair("code_challenge_method", "S256");
    Ok(url.into())
}

fn pkce_verifier() -> String {
    format!("{}{}", random_state(), random_state())
}

fn pkce_challenge(verifier: &str) -> String {
    base64url_no_pad(&Sha256::digest(verifier.as_bytes()))
}

fn base64url_no_pad(bytes: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity((bytes.len() * 4).div_ceil(3));
    let mut i = 0;
    while i + 3 <= bytes.len() {
        let n = u32::from(bytes[i]) << 16 | u32::from(bytes[i + 1]) << 8 | u32::from(bytes[i + 2]);
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(T[((n >> 6) & 63) as usize] as char);
        out.push(T[(n & 63) as usize] as char);
        i += 3;
    }
    if i < bytes.len() {
        let rem = bytes.len() - i;
        let mut n = u32::from(bytes[i]) << 16;
        if rem == 2 {
            n |= u32::from(bytes[i + 1]) << 8;
        }
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        if rem == 2 {
            out.push(T[((n >> 6) & 63) as usize] as char);
        }
    }
    out
}

fn token_fields<'a>(extra: &[(&'a str, &'a str)], secret: &'a str) -> Vec<(&'a str, &'a str)> {
    let mut fields = Vec::with_capacity(extra.len() + 2);
    fields.push(("client_id", TWITCH_CLIENT_ID));
    fields.extend_from_slice(extra);
    let secret = secret.trim();
    if !secret.is_empty() {
        fields.push(("client_secret", secret));
    }
    fields
}

fn twitch_form(url: &str, fields: &[(&str, &str)]) -> Result<Value, String> {
    match ureq::post(url)
        .set("Content-Type", "application/x-www-form-urlencoded")
        .timeout(Duration::from_secs(15))
        .send_form(fields)
    {
        Ok(resp) => resp.into_json().map_err(|e| format!("twitch oauth: {e}")),
        Err(ureq::Error::Status(code, resp)) => {
            let body = resp.into_string().unwrap_or_default();
            Err(format!("twitch oauth HTTP {code}: {body}"))
        }
        Err(e) => Err(format!("twitch oauth: {e}")),
    }
}

fn apply_token_payload(cfg: &mut Config, payload: &Value) -> Result<(), String> {
    let access = payload
        .get("access_token")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "twitch oauth: missing access_token".to_string())?;
    cfg.twitch_oauth = access.to_string();
    if let Some(refresh) = payload
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        cfg.twitch_refresh_token = refresh.to_string();
    }
    let expires_in = payload
        .get("expires_in")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    cfg.twitch_oauth_expires_at = if expires_in > 0 { now + expires_in } else { 0 };
    Ok(())
}

fn validate_login(access: &str) -> Result<String, String> {
    let token = bare_token(access);
    let resp = ureq::get("https://id.twitch.tv/oauth2/validate")
        .set("Authorization", &format!("OAuth {token}"))
        .timeout(Duration::from_secs(15))
        .call()
        .map_err(|e| format!("twitch validate: {e}"))?;
    let payload: Value = resp
        .into_json()
        .map_err(|e| format!("twitch validate: {e}"))?;
    payload
        .get("login")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "twitch validate: missing login".into())
}

pub fn exchange_code(
    store: &Store,
    code: &str,
    redirect_uri: &str,
    verifier: &str,
) -> Result<String, String> {
    let cfg = store.get()?;
    let code = code.trim().to_string();
    let secret = cfg.twitch_client_secret.clone();
    let fields = token_fields(
        &[
            ("code", code.as_str()),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri),
            ("code_verifier", verifier),
        ],
        &secret,
    );
    let payload = twitch_form("https://id.twitch.tv/oauth2/token", &fields)?;
    // Validate on a scratch copy (network), then write only token fields to the live config.
    let mut scratch = cfg;
    apply_token_payload(&mut scratch, &payload)?;
    let login = validate_login(&scratch.twitch_oauth)?;
    store.update(|c| {
        apply_token_payload(c, &payload)?;
        c.twitch_nick = login.clone();
        if c.twitch_channel.trim().is_empty() {
            c.twitch_channel = login.clone();
        }
        Ok(())
    })?;
    Ok(login)
}

/// Refresh the access token when it is missing or about to expire.
/// A refresh token Twitch rejects (HTTP 400/401) is wiped and `LOGIN_EXPIRED` returned,
/// so the worker stops retrying until the operator logs in again.
pub fn ensure_access_token(store: &Store, force: bool) -> Result<Config, String> {
    let cfg = store.get()?;
    if cfg.twitch_refresh_token.trim().is_empty() {
        return Ok(cfg);
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let still_good = !force
        && !cfg.twitch_oauth.trim().is_empty()
        && cfg.twitch_oauth_expires_at > now + TOKEN_SKEW_SECS;
    if still_good {
        return Ok(cfg);
    }
    let refresh = cfg.twitch_refresh_token.trim().to_string();
    let secret = cfg.twitch_client_secret.clone();
    let fields = token_fields(
        &[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh.as_str()),
        ],
        &secret,
    );
    let payload = match twitch_form("https://id.twitch.tv/oauth2/token", &fields) {
        Ok(p) => p,
        Err(e)
            if e.starts_with("twitch oauth HTTP 400") || e.starts_with("twitch oauth HTTP 401") =>
        {
            eprintln!("twitch: refresh rejected ({e}); clearing login");
            store.update(|c| {
                c.twitch_oauth.clear();
                c.twitch_refresh_token.clear();
                c.twitch_oauth_expires_at = 0;
                Ok(())
            })?;
            return Err(LOGIN_EXPIRED.into());
        }
        Err(e) => return Err(e),
    };
    let mut scratch = cfg;
    apply_token_payload(&mut scratch, &payload)?;
    let login = validate_login(&scratch.twitch_oauth).ok();
    store.update(|c| {
        apply_token_payload(c, &payload)?;
        if let Some(login) = &login {
            c.twitch_nick = login.clone();
        }
        Ok(())
    })
}

fn creds_from_config(cfg: &Config) -> Option<(String, String, String)> {
    let channel = cfg
        .twitch_channel
        .trim()
        .trim_start_matches('#')
        .to_lowercase();
    let oauth = cfg.twitch_oauth.trim().to_string();
    if channel.is_empty() || oauth.is_empty() {
        return None;
    }
    let nick = {
        let n = cfg.twitch_nick.trim();
        if n.is_empty() {
            channel.clone()
        } else {
            n.to_lowercase()
        }
    };
    Some((channel, oauth, nick))
}

fn sanitize_privmsg(text: &str) -> String {
    let cleaned: String = text.chars().filter(|c| *c != '\r' && *c != '\n').collect();
    cleaned.trim().chars().take(MAX_PRIVMSG).collect()
}

/// Unguessable OAuth `state` (128 bits). Each `RandomState` is seeded from OS randomness,
/// so this needs no extra crate.
fn random_state() -> String {
    let a = RandomState::new().hash_one(now_ms());
    let b = RandomState::new().hash_one(a);
    format!("{a:016x}{b:016x}")
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn irc_tag_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let tags = line.strip_prefix('@')?.split_once(' ')?.0;
    for pair in tags.split(';') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == key {
                return Some(v);
            }
        }
    }
    None
}

fn is_hex_color(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 7 && b[0] == b'#' && b[1..].iter().all(u8::is_ascii_hexdigit)
}

/// Twitch `color` tag; empty when missing, blank, or not `#RRGGBB`.
fn privmsg_color(line: &str) -> String {
    match irc_tag_value(line, "color") {
        Some(raw) if is_hex_color(raw) => raw.to_string(),
        _ => String::new(),
    }
}

fn parse_privmsg(line: &str) -> Option<(String, String)> {
    let without_tags = if line.starts_with('@') {
        line.split_once(' ')?.1
    } else {
        line
    };
    let rest = without_tags.strip_prefix(':')?;
    let (prefix, after_prefix) = rest.split_once(' ')?;
    if !after_prefix.to_ascii_uppercase().starts_with("PRIVMSG ") {
        return None;
    }
    let nick = prefix.split('!').next()?.to_string();
    if nick.is_empty() {
        return None;
    }
    let (_, body) = after_prefix.split_once(" :")?;
    Some((nick, body.to_string()))
}

fn parse_command(body: &str) -> Option<(String, String)> {
    let body = body.trim().strip_prefix('!')?;
    let mut parts = body.splitn(2, char::is_whitespace);
    let cmd = parts.next()?.to_ascii_lowercase();
    let rest = parts.next().unwrap_or("").trim().to_string();
    Some((cmd, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_untagged_privmsg_nick() {
        let (nick, body) =
            parse_privmsg(":botty!botty@botty.tmi.twitch.tv PRIVMSG #streamer :hello").unwrap();
        assert_eq!(nick, "botty");
        assert_eq!(body, "hello");
    }

    #[test]
    fn parse_tagged_privmsg_nick() {
        let line = "@badge-info= :alice!alice@alice.tmi.twitch.tv PRIVMSG #bob :!sr never gonna";
        let (nick, body) = parse_privmsg(line).unwrap();
        assert_eq!(nick, "alice");
        assert_eq!(body, "!sr never gonna");
    }

    #[test]
    fn parse_tagged_privmsg_color() {
        let line = "@badge-info=;color=#FF69B4;display-name=Alice :alice!alice@alice.tmi.twitch.tv PRIVMSG #bob :hi";
        assert_eq!(privmsg_color(line), "#FF69B4");
        let (nick, body) = parse_privmsg(line).unwrap();
        assert_eq!(nick, "alice");
        assert_eq!(body, "hi");
    }

    #[test]
    fn parse_empty_and_junk_color_is_blank() {
        let empty = "@color=;display-name=Bob :bob!bob@bob.tmi.twitch.tv PRIVMSG #c :yo";
        assert_eq!(privmsg_color(empty), "");
        let junk = "@color=red :bob!bob@bob.tmi.twitch.tv PRIVMSG #c :yo";
        assert_eq!(privmsg_color(junk), "");
        let untagged = ":bob!bob@bob.tmi.twitch.tv PRIVMSG #c :yo";
        assert_eq!(privmsg_color(untagged), "");
    }

    #[test]
    fn nick_can_differ_from_join_channel() {
        let cfg = Config {
            twitch_channel: "streamer".into(),
            twitch_oauth: "oauth:secret".into(),
            twitch_nick: "MyBot".into(),
            ..Config::default()
        };
        let (channel, _, nick) = creds_from_config(&cfg).unwrap();
        assert_eq!(channel, "streamer");
        assert_eq!(nick, "mybot");
    }

    #[test]
    fn empty_nick_defaults_to_channel() {
        let cfg = Config {
            twitch_channel: "#Streamer".into(),
            twitch_oauth: "token".into(),
            twitch_nick: "  ".into(),
            ..Config::default()
        };
        let (channel, _, nick) = creds_from_config(&cfg).unwrap();
        assert_eq!(channel, "streamer");
        assert_eq!(nick, "streamer");
    }

    #[test]
    fn sanitize_strips_crlf_and_caps() {
        assert_eq!(sanitize_privmsg(" hi\r\nthere "), "hithere");
        assert_eq!(sanitize_privmsg(&"a".repeat(600)).len(), MAX_PRIVMSG);
        assert!(sanitize_privmsg("  \n").is_empty());
    }

    #[test]
    fn aliases_map_to_canonical_ids() {
        assert_eq!(canonical_command("sr"), Some("sr"));
        assert_eq!(canonical_command("songrequest"), Some("sr"));
        assert_eq!(canonical_command("q"), Some("q"));
        assert_eq!(canonical_command("queue"), Some("q"));
        assert_eq!(canonical_command("si"), Some("si"));
        assert_eq!(canonical_command("songinfo"), Some("si"));
        assert_eq!(canonical_command("help"), None);
    }

    #[test]
    fn enqueue_skips_when_commands_off() {
        let hub = Hub::new();
        let mut cfg = Config::default();
        cfg.twitch_commands_enabled = false;
        assert!(!hub.try_enqueue_cmd(&cfg, "alice", "sr", "song"));
        assert_eq!(hub.pending_jobs(), 0);
    }

    #[test]
    fn gcd_blocks_everyone_ucd_blocks_user() {
        let hub = Hub::new();
        let mut cfg = Config::default();
        cfg.twitch_commands.get_mut("sr").unwrap().gcd = 30;
        assert!(hub.try_enqueue_cmd(&cfg, "alice", "sr", "a"));
        assert!(!hub.try_enqueue_cmd(&cfg, "bob", "sr", "b"));
        hub.clear_jobs();
        cfg.twitch_commands.get_mut("sr").unwrap().gcd = 0;
        cfg.twitch_commands.get_mut("sr").unwrap().ucd = 30;
        assert!(!hub.try_enqueue_cmd(&cfg, "alice", "sr", "c"));
        assert!(hub.try_enqueue_cmd(&cfg, "bob", "sr", "d"));
    }

    #[test]
    fn cmd_jobs_are_fifo() {
        let hub = Hub::new();
        let cfg = Config::default();
        assert!(hub.try_enqueue_cmd(&cfg, "alice", "sr", "a"));
        assert!(hub.try_enqueue_cmd(&cfg, "bob", "q", ""));
        assert!(hub.try_enqueue_cmd(&cfg, "carol", "si", ""));
        assert_eq!(hub.pop_job().unwrap().id, "sr");
        assert_eq!(hub.pop_job().unwrap().id, "q");
        assert_eq!(hub.pop_job().unwrap().id, "si");
        assert!(hub.pop_job().is_none());
    }

    #[test]
    fn cmd_queue_drops_newest_at_cap() {
        let hub = Hub::new();
        let cfg = Config::default();
        for i in 0..MAX_CMD_JOBS {
            assert!(hub.try_enqueue_cmd(&cfg, "alice", "sr", &i.to_string()));
        }
        assert_eq!(hub.pending_jobs(), MAX_CMD_JOBS);
        assert!(!hub.try_enqueue_cmd(&cfg, "bob", "sr", "too-many"));
        assert_eq!(hub.pending_jobs(), MAX_CMD_JOBS);
        assert_eq!(hub.pop_job().unwrap().args, "0");
        assert_eq!(hub.pop_job().unwrap().args, "1");
    }

    #[test]
    fn gcd_stamps_on_accept() {
        let hub = Hub::new();
        let mut cfg = Config::default();
        cfg.twitch_commands.get_mut("sr").unwrap().gcd = 30;
        assert!(hub.try_enqueue_cmd(&cfg, "alice", "sr", "a"));
        assert!(!hub.try_enqueue_cmd(&cfg, "bob", "sr", "b"));
        assert_eq!(hub.pending_jobs(), 1);
    }

    #[test]
    fn cooldown_zero_never_blocks() {
        assert!(!cooldown_hit(Some(Instant::now()), Instant::now(), 0));
        assert!(!cooldown_hit(None, Instant::now(), 10));
    }

    #[test]
    fn authorize_url_includes_scopes_state_and_pkce() {
        let url = authorize_url(
            "http://localhost:42042/api/twitch/callback",
            "st-1",
            "challenge-abc",
        )
        .unwrap();
        assert!(url.contains(&format!("client_id={TWITCH_CLIENT_ID}")));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("chat%3Aread"));
        assert!(url.contains("force_verify=true"));
        assert!(url.contains("state=st-1"));
        assert!(url.contains("code_challenge=challenge-abc"));
        assert!(url.contains("code_challenge_method=S256"));
    }

    #[test]
    fn pkce_challenge_matches_rfc7636() {
        assert_eq!(
            pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn oauth_start_roundtrip_keeps_verifier() {
        let hub = Hub::new();
        let redirect = "http://localhost:42042/api/twitch/callback";
        let url = hub.begin_oauth(redirect).unwrap();
        let parsed = Url::parse(&url).unwrap();
        let state = parsed
            .query_pairs()
            .find(|(k, _)| k == "state")
            .map(|(_, v)| v.into_owned())
            .unwrap();
        let challenge = parsed
            .query_pairs()
            .find(|(k, _)| k == "code_challenge")
            .map(|(_, v)| v.into_owned())
            .unwrap();
        let (got_redirect, verifier) = hub.take_oauth_redirect(&state).unwrap();
        assert_eq!(got_redirect, redirect);
        assert_eq!(pkce_challenge(&verifier), challenge);
    }

    #[test]
    fn apply_token_payload_sets_access_and_refresh() {
        let mut cfg = Config::default();
        apply_token_payload(
            &mut cfg,
            &json!({
                "access_token": "acc",
                "refresh_token": "ref",
                "expires_in": 14400
            }),
        )
        .unwrap();
        assert_eq!(cfg.twitch_oauth, "acc");
        assert_eq!(cfg.twitch_refresh_token, "ref");
        assert!(cfg.twitch_oauth_expires_at > 0);
    }

    #[test]
    fn oauth_state_is_not_repeated() {
        let a = random_state();
        assert_eq!(a.len(), 32);
        assert_ne!(a, random_state());
    }

    #[test]
    fn bare_token_strips_prefix() {
        assert_eq!(bare_token("oauth:abc"), "abc");
        assert_eq!(bare_token("abc"), "abc");
    }

    #[test]
    fn auto_connect_needs_channel_and_token_or_refresh() {
        let mut cfg = Config::default();
        cfg.twitch_channel = "streamer".into();
        assert!(!can_auto_connect(&cfg));
        cfg.twitch_refresh_token = "r".into();
        assert!(can_auto_connect(&cfg));
    }

    #[test]
    fn token_fields_omit_empty_secret() {
        let fields = token_fields(&[("grant_type", "refresh_token")], "  ");
        assert!(fields.iter().any(|(k, v)| *k == "client_id" && *v == TWITCH_CLIENT_ID));
        assert!(fields.iter().any(|(k, v)| *k == "grant_type" && *v == "refresh_token"));
        assert!(!fields.iter().any(|(k, _)| *k == "client_secret"));
    }

    #[test]
    fn token_fields_include_secret_when_set() {
        let fields = token_fields(&[("code", "abc")], "s3cret");
        assert_eq!(
            fields
                .iter()
                .find(|(k, _)| *k == "client_secret")
                .map(|(_, v)| *v),
            Some("s3cret")
        );
        assert_eq!(fields[0], ("client_id", TWITCH_CLIENT_ID));
    }

    #[test]
    fn welcome_line_is_numeric_001() {
        assert!(is_welcome(":tmi.twitch.tv 001 botty :Welcome, GLHF!"));
        assert!(!is_welcome(":tmi.twitch.tv 002 botty :your host"));
        assert!(!is_welcome("PING :tmi.twitch.tv"));
    }
}
