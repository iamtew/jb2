use crate::config::{Config, Store};
use crate::envelope::Envelope;
use crate::server::App;
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, Message, WebSocket};
use url::Url;

type Ws = WebSocket<MaybeTlsStream<TcpStream>>;

const CHAT_CAP: usize = 200;
const MAX_PRIVMSG: usize = 500;
const OAUTH_SCOPES: &str = "chat:read chat:edit";
const TOKEN_SKEW_SECS: i64 = 120;
/// Wait between retries after a token or repeated IRC auth failure, so a bad login
/// cannot hammer id.twitch.tv / irc-ws.chat.twitch.tv.
const AUTH_BACKOFF_SECS: u64 = 30;
/// Wait between retries after a network / server-closed error.
const RECONNECT_SECS: u64 = 10;
pub const LOGIN_EXPIRED: &str = "Twitch login expired; log in again";

pub struct Hub {
    inner: Mutex<Inner>,
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
    oauth_nonce: u64,
    oauth_state: Option<(String, String)>,
}

#[derive(Clone)]
struct ChatLine {
    id: u64,
    ts: u64,
    nick: String,
    text: String,
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
                oauth_nonce: 1,
                oauth_state: None,
            }),
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

    fn push_chat(&self, nick: &str, text: &str) {
        let mut s = self.inner.lock().expect("twitch");
        let id = s.next_id;
        s.next_id = s.next_id.saturating_add(1);
        s.chat.push_back(ChatLine {
            id,
            ts: now_ms(),
            nick: nick.to_string(),
            text: text.to_string(),
        });
        while s.chat.len() > CHAT_CAP {
            s.chat.pop_front();
        }
    }

    fn take_outbound(&self) -> Vec<String> {
        let mut s = self.inner.lock().expect("twitch");
        s.outbound.drain(..).collect()
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
        s.outbound.push_back(text);
        Ok(())
    }

    pub fn begin_oauth(&self, client_id: &str, redirect_uri: &str) -> Result<String, String> {
        let client_id = client_id.trim();
        if client_id.is_empty() {
            return Err("save twitch Client ID first".into());
        }
        let mut s = self.inner.lock().expect("twitch");
        s.oauth_nonce = s.oauth_nonce.saturating_add(1);
        let state = format!("{:x}-{}", now_ms(), s.oauth_nonce);
        s.oauth_state = Some((state.clone(), redirect_uri.to_string()));
        authorize_url(client_id, redirect_uri, &state)
    }

    pub fn take_oauth_redirect(&self, state: &str) -> Result<String, String> {
        let mut s = self.inner.lock().expect("twitch");
        match s.oauth_state.take() {
            Some((want, redirect)) if want == state => Ok(redirect),
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
            "clientIdSet": !cfg.twitch_client_id.trim().is_empty(),
            "secretSet": !cfg.twitch_client_secret.trim().is_empty(),
            "clientId": cfg.twitch_client_id.trim(),
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
        .spawn(move || worker(app))
        .expect("twitch thread");
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
    let (mut ws, _) = connect("wss://irc-ws.chat.twitch.tv:443").map_err(|e| e.to_string())?;
    set_read_timeout(&ws, Duration::from_millis(250))?;
    send(
        &mut ws,
        &format!("PASS {token}\r\nNICK {nick}\r\nJOIN #{channel}\r\n"),
    )?;
    app.twitch.set_session(true, channel, nick);
    eprintln!("twitch: joined #{channel} as {nick}");
    loop {
        if !app.twitch.wanted() || app.twitch.generation() != gen {
            let _ = ws.close(None);
            return Ok(());
        }
        for text in app.twitch.take_outbound() {
            send(&mut ws, &format!("PRIVMSG #{channel} :{text}\r\n"))?;
            app.twitch.push_chat(nick, &text);
        }
        match ws.read() {
            Ok(Message::Text(text)) => {
                for line in text.split(['\r', '\n']) {
                    handle_line(app, channel, nick, line, &mut ws)?;
                }
            }
            Ok(Message::Ping(p)) => {
                let _ = ws.send(Message::Pong(p));
            }
            Ok(Message::Close(_)) => return Err("server closed".into()),
            Ok(_) => {}
            Err(tungstenite::Error::Io(e))
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(e) => return Err(e.to_string()),
        }
    }
}

fn set_read_timeout(ws: &Ws, timeout: Duration) -> Result<(), String> {
    match ws.get_ref() {
        MaybeTlsStream::NativeTls(t) => t
            .get_ref()
            .set_read_timeout(Some(timeout))
            .map_err(|e| e.to_string()),
        MaybeTlsStream::Plain(t) => t.set_read_timeout(Some(timeout)).map_err(|e| e.to_string()),
        _ => Ok(()),
    }
}

fn send(ws: &mut Ws, s: &str) -> Result<(), String> {
    ws.send(Message::Text(s.to_string()))
        .map_err(|e| e.to_string())
}

fn handle_line(
    app: &Arc<App>,
    channel: &str,
    bot_nick: &str,
    line: &str,
    ws: &mut Ws,
) -> Result<(), String> {
    let line = line.trim();
    if line.is_empty() {
        return Ok(());
    }
    if line.starts_with("PING") {
        return send(ws, &format!("{}\r\n", line.replacen("PING", "PONG", 1)));
    }
    if is_auth_fail(line) {
        return Err(line.to_string());
    }
    let Some((from, body)) = parse_privmsg(line) else {
        return Ok(());
    };
    app.twitch.push_chat(&from, &body);
    let Some((cmd, rest)) = parse_command(&body) else {
        return Ok(());
    };
    let reply = match cmd.as_str() {
        "songrequest" | "sr" => {
            if rest.is_empty() {
                "usage: !sr <song>".to_string()
            } else {
                envelope_text(app.song_request(&rest, false).envelope())
            }
        }
        "queue" | "q" => envelope_text(app.queue_info().envelope()),
        "songinfo" | "si" => envelope_text(app.song_info().envelope()),
        _ => return Ok(()),
    };
    send(ws, &format!("PRIVMSG #{channel} :{reply}\r\n"))?;
    app.twitch.push_chat(bot_nick, &reply);
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

fn authorize_url(client_id: &str, redirect_uri: &str, state: &str) -> Result<String, String> {
    let mut url = Url::parse("https://id.twitch.tv/oauth2/authorize").map_err(|e| e.to_string())?;
    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", OAUTH_SCOPES)
        .append_pair("state", state)
        .append_pair("force_verify", "true");
    Ok(url.into())
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

pub fn exchange_code(store: &Store, code: &str, redirect_uri: &str) -> Result<String, String> {
    let cfg = store.get()?;
    let client_id = cfg.twitch_client_id.trim().to_string();
    let client_secret = cfg.twitch_client_secret.trim().to_string();
    if client_id.is_empty() || client_secret.is_empty() {
        return Err("save twitch Client ID and Secret first".into());
    }
    let payload = twitch_form(
        "https://id.twitch.tv/oauth2/token",
        &[
            ("client_id", &client_id),
            ("client_secret", &client_secret),
            ("code", code.trim()),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri),
        ],
    )?;
    // Validate on a scratch copy (network), then write only token fields to the live config.
    let mut scratch = cfg;
    apply_token_payload(&mut scratch, &payload)?;
    let login = validate_login(&scratch.twitch_oauth)?;
    store.update(|c| {
        apply_token_payload(c, &payload)?;
        c.twitch_nick = login.clone();
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
    if cfg.twitch_client_id.trim().is_empty() || cfg.twitch_client_secret.trim().is_empty() {
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
    let payload = match twitch_form(
        "https://id.twitch.tv/oauth2/token",
        &[
            ("client_id", cfg.twitch_client_id.trim()),
            ("client_secret", cfg.twitch_client_secret.trim()),
            ("grant_type", "refresh_token"),
            ("refresh_token", cfg.twitch_refresh_token.trim()),
        ],
    ) {
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

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
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
    fn authorize_url_includes_scopes_and_state() {
        let url = authorize_url(
            "abc123",
            "http://localhost:42042/api/twitch/callback",
            "st-1",
        )
        .unwrap();
        assert!(url.contains("client_id=abc123"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("chat%3Aread"));
        assert!(url.contains("force_verify=true"));
        assert!(url.contains("state=st-1"));
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
}
