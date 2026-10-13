use crate::envelope::Envelope;
use crate::server::App;
use std::sync::Arc;
use std::time::Duration;
use tungstenite::{connect, Message, WebSocket};
use tungstenite::stream::MaybeTlsStream;
use std::net::TcpStream;

type Ws = WebSocket<MaybeTlsStream<TcpStream>>;

pub fn spawn(app: Arc<App>) {
    let cfg = match app.store.get() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("twitch: config: {e}");
            return;
        }
    };
    let channel = cfg.twitch_channel.trim().trim_start_matches('#').to_lowercase();
    let oauth = cfg.twitch_oauth.trim().to_string();
    if channel.is_empty() || oauth.is_empty() {
        eprintln!("twitch: skipped (set twitchChannel + twitchOAuth in jb2.json)");
        return;
    }
    let nick = {
        let n = cfg.twitch_nick.trim();
        if n.is_empty() {
            channel.clone()
        } else {
            n.to_lowercase()
        }
    };
    std::thread::Builder::new()
        .name("twitch".into())
        .spawn(move || loop {
            if let Err(e) = run(&app, &channel, &oauth, &nick) {
                eprintln!("twitch: {e}; retry in 10s");
            }
            std::thread::sleep(Duration::from_secs(10));
        })
        .expect("twitch thread");
}

fn run(app: &Arc<App>, channel: &str, oauth: &str, nick: &str) -> Result<(), String> {
    let token = if oauth.starts_with("oauth:") {
        oauth.to_string()
    } else {
        format!("oauth:{oauth}")
    };
    let (mut ws, _) = connect("wss://irc-ws.chat.twitch.tv:443").map_err(|e| e.to_string())?;
    send(&mut ws, &format!("PASS {token}\r\nNICK {nick}\r\nJOIN #{channel}\r\n"))?;
    eprintln!("twitch: joined #{channel} as {nick}");
    loop {
        match ws.read().map_err(|e| e.to_string())? {
            Message::Text(text) => {
                for line in text.split(['\r', '\n']) {
                    handle_line(app, channel, line, &mut ws)?;
                }
            }
            Message::Ping(p) => {
                let _ = ws.send(Message::Pong(p));
            }
            Message::Close(_) => return Err("server closed".into()),
            _ => {}
        }
    }
}

fn send(ws: &mut Ws, s: &str) -> Result<(), String> {
    ws.send(Message::Text(s.to_string())).map_err(|e| e.to_string())
}

fn handle_line(app: &Arc<App>, channel: &str, line: &str, ws: &mut Ws) -> Result<(), String> {
    let line = line.trim();
    if line.is_empty() {
        return Ok(());
    }
    if line.starts_with("PING") {
        return send(ws, &format!("{}\r\n", line.replacen("PING", "PONG", 1)));
    }
    let Some(privmsg) = parse_privmsg(line) else {
        return Ok(());
    };
    let Some((cmd, rest)) = parse_command(&privmsg) else {
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
    send(ws, &format!("PRIVMSG #{channel} :{reply}\r\n"))
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

fn parse_privmsg(line: &str) -> Option<String> {
    let without_tags = if line.starts_with('@') {
        line.split_once(' ')?.1
    } else {
        line
    };
    let rest = without_tags.strip_prefix(':')?;
    let (_, after_prefix) = rest.split_once(' ')?;
    if !after_prefix.to_ascii_uppercase().starts_with("PRIVMSG ") {
        return None;
    }
    let (_, body) = after_prefix.split_once(" :")?;
    Some(body.to_string())
}

fn parse_command(body: &str) -> Option<(String, String)> {
    let body = body.trim().strip_prefix('!')?;
    let mut parts = body.splitn(2, char::is_whitespace);
    let cmd = parts.next()?.to_ascii_lowercase();
    let rest = parts.next().unwrap_or("").trim().to_string();
    Some((cmd, rest))
}
