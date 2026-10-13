#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod config;
mod envelope;
mod httpctl;
mod jsonutil;
mod policy;
mod seed;
mod server;
mod twitch;
mod videoid;
mod ytmd;

#[cfg(windows)]
mod tray;

use server::App;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn main() {
    attach_parent_console();

    let args: Vec<String> = env::args().collect();
    let port = flag(&args, "-port").unwrap_or("42042".into());
    let ytmd_host = flag(&args, "-ytmd_host").unwrap_or("localhost".into());
    let ytmd_port = flag(&args, "-ytmd_port").unwrap_or("26538".into());
    let bind = flag(&args, "-bind").unwrap_or("0.0.0.0".into());
    let webroot = resolve_path(flag(&args, "-webroot"), "webroot");
    let config_path = resolve_path(flag(&args, "-config"), "jb2.sqlite");

    if !webroot.is_dir() {
        fatal(&format!("webroot not found at {}", webroot.display()));
    }

    let store = match config::Store::new(config_path.clone()) {
        Ok(s) => s,
        Err(e) => fatal(&format!("failed to initialize config: {e}")),
    };
    let client = ytmd::Client::new(&ytmd_host, &ytmd_port);
    let app = App::new(store, client, webroot.clone());
    twitch::spawn(Arc::clone(&app));

    let port_n: u16 = match port.parse() {
        Ok(n) => n,
        Err(_) => fatal(&format!("invalid port {port}")),
    };
    httpctl::init(app, bind, port_n);
    if let Err(e) = httpctl::start() {
        fatal(&format!("failed to start server: {e}"));
    }
    eprintln!("webroot: {}", webroot.display());
    eprintln!("config:  {}", config_path.display());

    wait_for_quit();
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.windows(2).find_map(|w| {
        if w[0] == name {
            Some(w[1].clone())
        } else {
            None
        }
    })
}

fn resolve_path(explicit: Option<String>, name: &str) -> PathBuf {
    if let Some(p) = explicit {
        if !p.trim().is_empty() {
            return PathBuf::from(p);
        }
    }
    if let Some(dir) = deployed_dir() {
        return dir.join(name);
    }
    PathBuf::from(name)
}

fn deployed_dir() -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    let dir = exe.parent()?;
    if dir.join("webroot").is_dir() {
        Some(dir.to_path_buf())
    } else {
        None
    }
}

fn wait_for_quit() {
    #[cfg(windows)]
    tray::wait_for_quit();
    #[cfg(not(windows))]
    {
        let (tx, rx) = std::sync::mpsc::channel();
        let _ = ctrlc_stub(tx);
        let _ = rx.recv();
        httpctl::stop();
    }
}

#[cfg(not(windows))]
fn ctrlc_stub(tx: std::sync::mpsc::Sender<()>) -> Result<(), ()> {
    std::thread::spawn(move || {
        let mut line = String::new();
        let _ = std::io::stdin().read_line(&mut line);
        let _ = tx.send(());
    });
    Ok(())
}

fn attach_parent_console() {
    #[cfg(windows)]
    unsafe {
        windows::Win32::System::Console::AttachConsole(
            windows::Win32::System::Console::ATTACH_PARENT_PROCESS,
        )
        .ok();
    }
}

pub fn alert(msg: &str) {
    eprintln!("{msg}");
    #[cfg(windows)]
    unsafe {
        use windows::core::HSTRING;
        use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
        let _ = MessageBoxW(
            None,
            &HSTRING::from(msg),
            &HSTRING::from("JB2"),
            MB_OK | MB_ICONERROR,
        );
    }
}

pub fn fatal(msg: &str) -> ! {
    alert(msg);
    std::process::exit(1);
}

#[allow(dead_code)]
fn _path_as_path(p: &Path) -> &Path {
    p
}
