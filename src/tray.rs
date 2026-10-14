#![cfg(windows)]

use crate::httpctl;
use std::mem::size_of;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::Shell::{
    ExtractIconExW, ShellExecuteW, Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD,
    NIM_DELETE, NOTIFYICONDATAW, NOTIFYICONDATAW_0,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DispatchMessageW,
    GetCursorPos, GetMessageW, LoadCursorW, LoadIconW, PostMessageW, PostQuitMessage,
    RegisterClassExW, SetForegroundWindow, TrackPopupMenu, TranslateMessage, CS_HREDRAW,
    CS_VREDRAW, HICON, HMENU, IDC_ARROW, IDI_APPLICATION, MF_SEPARATOR, MF_STRING, MSG,
    SW_SHOWNORMAL, TPM_RETURNCMD, TPM_RIGHTBUTTON, WINDOW_EX_STYLE, WM_APP, WM_CONTEXTMENU,
    WM_DESTROY, WM_LBUTTONUP, WM_RBUTTONUP, WNDCLASSEXW, WS_OVERLAPPED,
};

const WM_TRAY: u32 = WM_APP + 1;
const ID_OPEN_HOME: usize = 1001;
const ID_OPEN_ADMIN: usize = 1002;
const ID_OPEN_OVERLAY: usize = 1003;
const ID_ABOUT: usize = 1004;
const ID_RESTART: usize = 1005;
const ID_QUIT: usize = 1006;

pub fn wait_for_quit() {
    unsafe {
        let _mutex = CreateMutexW(None, true, w!("Local\\JB2SingleInstance"));
        if windows::Win32::Foundation::GetLastError()
            == windows::Win32::Foundation::ERROR_ALREADY_EXISTS
        {
            crate::fatal("JB2 is already running");
        }

        let instance = GetModuleHandleW(None).expect("module");
        let icon = load_app_icon();
        let class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(tray_proc),
            hInstance: instance.into(),
            hIcon: icon,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            lpszClassName: w!("JB2Tray"),
            hIconSm: icon,
            ..Default::default()
        };
        RegisterClassExW(&class);
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("JB2Tray"),
            w!("JB2"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            instance,
            None,
        )
        .expect("tray window");

        let mut nid = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: WM_TRAY,
            hIcon: icon,
            Anonymous: NOTIFYICONDATAW_0::default(),
            ..Default::default()
        };
        copy_tip(&mut nid.szTip, "JB2");
        if !Shell_NotifyIconW(NIM_ADD, &nid).as_bool() {
            crate::fatal("failed to add tray icon");
        }

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
        httpctl::stop();
    }
}

unsafe extern "system" fn tray_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_TRAY => {
            let ev = lparam.0 as u32;
            if ev == WM_LBUTTONUP || ev == WM_RBUTTONUP || ev == WM_CONTEXTMENU {
                show_menu(hwnd);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn show_menu(hwnd: HWND) {
    unsafe {
        let menu = CreatePopupMenu().expect("menu");
        let _ = AppendMenuW(menu, MF_STRING, ID_OPEN_HOME, w!("Open Home"));
        let _ = AppendMenuW(menu, MF_STRING, ID_OPEN_ADMIN, w!("Open Admin"));
        let _ = AppendMenuW(menu, MF_STRING, ID_OPEN_OVERLAY, w!("Open Overlay"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING, ID_ABOUT, w!("About"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING, ID_RESTART, w!("Restart"));
        let _ = AppendMenuW(menu, MF_STRING, ID_QUIT, w!("Quit"));
        let mut pt = windows::Win32::Foundation::POINT::default();
        let _ = GetCursorPos(&mut pt);
        let _ = SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenu(
            menu,
            TPM_RIGHTBUTTON | TPM_RETURNCMD,
            pt.x,
            pt.y,
            0,
            hwnd,
            None,
        );
        let _ = PostMessageW(hwnd, 0, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(menu);
        match cmd.0 as usize {
            ID_OPEN_HOME => open_url(&httpctl::serving_url("/")),
            ID_OPEN_ADMIN => open_url(&httpctl::serving_url("/admin/")),
            ID_OPEN_OVERLAY => open_url(&httpctl::serving_url("/overlay/")),
            ID_ABOUT => show_about(hwnd),
            ID_RESTART => httpctl::restart(),
            ID_QUIT => PostQuitMessage(0),
            _ => {}
        }
        let _ = HMENU(menu.0);
    }
}

fn load_app_icon() -> HICON {
    unsafe {
        if let Ok(exe) = std::env::current_exe() {
            let wide: Vec<u16> = exe
                .to_string_lossy()
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let mut large = HICON::default();
            let mut small = HICON::default();
            let n = ExtractIconExW(
                PCWSTR(wide.as_ptr()),
                0,
                Some(&mut large),
                Some(&mut small),
                1,
            );
            if n > 0 && !large.is_invalid() {
                return large;
            }
            if !small.is_invalid() {
                return small;
            }
        }
        LoadIconW(None, IDI_APPLICATION).unwrap_or_default()
    }
}

fn show_about(hwnd: HWND) {
    unsafe {
        use windows::core::HSTRING;
        use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK};
        let body = format!("Jukeboks 2\nrunning\n{}", crate::version());
        let _ = MessageBoxW(
            hwnd,
            &HSTRING::from(body),
            &HSTRING::from("About"),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}

fn open_url(url: &str) {
    unsafe {
        let wide: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
        ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
    }
}

fn copy_tip(dst: &mut [u16], s: &str) {
    let v: Vec<u16> = s.encode_utf16().collect();
    let n = v.len().min(dst.len().saturating_sub(1));
    dst[..n].copy_from_slice(&v[..n]);
    dst[n] = 0;
}
