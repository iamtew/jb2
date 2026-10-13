use crate::config::Config;
use crate::envelope::Envelope;
use crate::server::App;
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, Message, WebSocket};

type Ws = WebSocket<MaybeTlsStream<TcpStream>>;

const CHAT_CAP: usize = 200;
const MAX_PRIVMSG: usize = 500;

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

    /// Drop the live IRC socket so the worker re-reads jb2.json.
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

    pub fn status_json(&self, oauth_set: bool) -> Value {
        let s = self.inner.lock().expect("twitch");
        json!({
            "connected": s.connected,
            "wanted": s.wanted,
            "channel": s.channel,
            "nick": s.nick,
            "lastError": s.last_error,
            "oauthSet": oauth_set,
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
        if creds_from_config(&cfg).is_some() {
            app.twitch.set_wanted(true);
        }
    }
    std::thread::Builder::new()
        .name("twitch".into())
        .spawn(move || worker(app))
        .expect("twitch thread");
}

fn worker(app: Arc<App>) {
    loop {
        if !app.twitch.wanted() {
            app.twitch.set_session(false, "", "");
            std::thread::sleep(Duration::from_millis(200));
            continue;
        }
        let Some((channel, oauth, nick)) = app.store.get().ok().and_then(|c| creds_from_config(&c)) else {
            app.twitch.set_error("set twitchChannel + twitchOAuth");
            std::thread::sleep(Duration::from_millis(500));
            continue;
        };
        let gen = app.twitch.generation();
        match run(&app, &channel, &oauth, &nick, gen) {
            Ok(()) => {}
            Err(e) => {
                app.twitch.set_error(&e);
                eprintln!("twitch: {e}; retry in 10s");
                // ponytail: 200ms polls instead of one 10s sleep so disconnect is snappy
                for _ in 0..50 {
                    if !app.twitch.wanted() || app.twitch.generation() != gen {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(200));
                }
            }
        }
        app.twitch.set_session(false, &channel, &nick);
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
    ws.send(Message::Text(s.to_string())).map_err(|e| e.to_string())
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
    let cleaned: String = text
        .chars()
        .filter(|c| *c != '\r' && *c != '\n')
        .collect();
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
        let (nick, body) = parse_privmsg(":botty!botty@botty.tmi.twitch.tv PRIVMSG #streamer :hello").unwrap();
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
}
