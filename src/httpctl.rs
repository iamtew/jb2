use crate::server::App;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use tiny_http::Server;

struct Runtime {
    server: Arc<Server>,
    worker: Option<JoinHandle<()>>,
}

static STATE: Mutex<Option<Runtime>> = Mutex::new(None);
static APP: Mutex<Option<Arc<App>>> = Mutex::new(None);
static PREFERRED_PORT: Mutex<u16> = Mutex::new(42042);
static ACTUAL_PORT: Mutex<u16> = Mutex::new(42042);
static BIND: Mutex<String> = Mutex::new(String::new());

pub fn init(app: Arc<App>, bind: String, port: u16) {
    *APP.lock().expect("app") = Some(app);
    *BIND.lock().expect("bind") = bind;
    *PREFERRED_PORT.lock().expect("port") = port;
}

pub fn start() -> Result<(), String> {
    let mut guard = STATE.lock().expect("http");
    if guard.is_some() {
        return Ok(());
    }
    let preferred = *PREFERRED_PORT.lock().expect("port");
    let bind = BIND.lock().expect("bind").clone();
    let (listener, port) = listen_with_fallback(&bind, preferred)?;
    *ACTUAL_PORT.lock().expect("port") = port;
    let server = Arc::new(Server::from_listener(listener, None).map_err(|e| e.to_string())?);
    let app = APP
        .lock()
        .expect("app")
        .clone()
        .ok_or("app not initialized")?;
    let srv = server.clone();
    let worker = std::thread::spawn(move || {
        for request in srv.incoming_requests() {
            let app = app.clone();
            std::thread::spawn(move || app.handle(request));
        }
    });
    *guard = Some(Runtime {
        server,
        worker: Some(worker),
    });
    eprintln!("listening on http://127.0.0.1:{port}");
    Ok(())
}

pub fn stop() {
    let mut guard = STATE.lock().expect("http");
    if let Some(mut rt) = guard.take() {
        rt.server.unblock();
        if let Some(h) = rt.worker.take() {
            let _ = h.join();
        }
    }
}

pub fn restart() {
    stop();
    if let Err(e) = start() {
        crate::alert(&format!("failed to restart server: {e}"));
    }
}

pub fn serving_url(path: &str) -> String {
    let port = *ACTUAL_PORT.lock().expect("port");
    format!("http://127.0.0.1:{port}{path}")
}

fn listen_with_fallback(bind: &str, base: u16) -> Result<(TcpListener, u16), String> {
    for attempt in 0..10u16 {
        let port = base + attempt;
        match TcpListener::bind(format!("{bind}:{port}")) {
            Ok(l) => {
                if port != base {
                    eprintln!("port {base} was busy, using {port} instead");
                }
                return Ok((l, port));
            }
            Err(_) => continue,
        }
    }
    Err(format!(
        "unable to bind to port {base} or any fallback ports"
    ))
}
