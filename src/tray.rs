#![cfg(windows)]

use crate::httpctl;
use std::mem::size_of;
use windows::core::{w, PCWSTR, PWSTR};
use windows::Win32::Foundation::{GlobalFree, HANDLE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateDIBSection, DeleteObject, GetSysColor, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    COLOR_MENUTEXT, DIB_RGB_COLORS, HBITMAP, HDC,
};
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_WICPixelFormat32bppBGRA, IWICImagingFactory, IWICPalette,
    WICBitmapDitherTypeNone, WICBitmapInterpolationModeFant, WICBitmapPaletteTypeCustom,
    WICDecodeMetadataCacheOnDemand,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::Shell::{
    ExtractIconExW, ShellExecuteW, Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD,
    NIM_DELETE, NOTIFYICONDATAW, NOTIFYICONDATAW_0,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DispatchMessageW,
    GetCursorPos, GetMessageW, GetSystemMetrics, InsertMenuItemW, LoadCursorW, LoadIconW,
    PostMessageW, PostQuitMessage, RegisterClassExW, SetForegroundWindow, TrackPopupMenu,
    TranslateMessage, CS_HREDRAW, CS_VREDRAW, HICON, HMENU, IDC_ARROW, IDI_APPLICATION,
    MENUITEMINFOW, MF_SEPARATOR, MIIM_BITMAP, MIIM_ID, MIIM_STRING, MSG, SM_CXSMICON,
    SW_SHOWNORMAL, TPM_RETURNCMD, TPM_RIGHTBUTTON, WINDOW_EX_STYLE, WM_APP, WM_CONTEXTMENU,
    WM_DESTROY, WM_LBUTTONUP, WM_RBUTTONUP, WNDCLASSEXW, WS_OVERLAPPED,
};

const WM_TRAY: u32 = WM_APP + 1;
const ID_OPEN_HOME: usize = 1001;
const ID_OPEN_ADMIN: usize = 1002;
const ID_OPEN_OVERLAY: usize = 1003;
const ID_OPEN_STAGE: usize = 1007;
const ID_COPY_OVERLAY: usize = 1008;
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

        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
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
        let icons = menu_icons();
        let mut pos = 0u32;
        append_item(menu, &mut pos, ID_OPEN_HOME, "Home", icons.home);
        append_item(menu, &mut pos, ID_OPEN_ADMIN, "Control Panel", icons.admin);
        append_item(menu, &mut pos, ID_OPEN_STAGE, "Overlay Stage", icons.stage);
        append_item(menu, &mut pos, ID_OPEN_OVERLAY, "Overlay", icons.overlay);
        append_item(
            menu,
            &mut pos,
            ID_COPY_OVERLAY,
            "Copy Overlay URL",
            icons.copy,
        );
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        pos += 1;
        append_item(menu, &mut pos, ID_ABOUT, "About", icons.about);
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        pos += 1;
        append_item(menu, &mut pos, ID_RESTART, "Restart", icons.restart);
        append_item(menu, &mut pos, ID_QUIT, "Quit", icons.quit);
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
        icons.free();
        match cmd.0 as usize {
            ID_OPEN_HOME => open_url(&httpctl::serving_url("/")),
            ID_OPEN_ADMIN => open_url(&httpctl::serving_url("/admin/")),
            ID_OPEN_OVERLAY => open_url(&httpctl::serving_url("/overlay/")),
            ID_COPY_OVERLAY => copy_text(hwnd, &httpctl::serving_url("/overlay/")),
            ID_OPEN_STAGE => open_url(&httpctl::serving_url("/stage/")),
            ID_ABOUT => show_about(hwnd),
            ID_RESTART => httpctl::restart(),
            ID_QUIT => PostQuitMessage(0),
            _ => {}
        }
        let _ = HMENU(menu.0);
    }
}

fn append_item(menu: HMENU, pos: &mut u32, id: usize, text: &str, bmp: HBITMAP) {
    let mut wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let info = MENUITEMINFOW {
        cbSize: size_of::<MENUITEMINFOW>() as u32,
        fMask: MIIM_ID | MIIM_STRING | MIIM_BITMAP,
        wID: id as u32,
        dwTypeData: PWSTR(wide.as_mut_ptr()),
        cch: (wide.len() - 1) as u32,
        hbmpItem: bmp,
        ..Default::default()
    };
    unsafe {
        let _ = InsertMenuItemW(menu, *pos, true, &info);
    }
    *pos += 1;
}

struct MenuIcons {
    home: HBITMAP,
    admin: HBITMAP,
    stage: HBITMAP,
    overlay: HBITMAP,
    copy: HBITMAP,
    about: HBITMAP,
    restart: HBITMAP,
    quit: HBITMAP,
}

impl MenuIcons {
    fn free(self) {
        for bmp in [
            self.home,
            self.admin,
            self.stage,
            self.overlay,
            self.copy,
            self.about,
            self.restart,
            self.quit,
        ] {
            if !bmp.is_invalid() {
                unsafe {
                    let _ = DeleteObject(bmp);
                }
            }
        }
    }
}

// Rebuild each popup so COLOR_MENUTEXT tracks the theme. DestroyMenu does not free hbmpItem.
fn menu_icons() -> MenuIcons {
    let factory = unsafe {
        CoCreateInstance::<_, IWICImagingFactory>(
            &CLSID_WICImagingFactory,
            None,
            CLSCTX_INPROC_SERVER,
        )
        .ok()
    };
    MenuIcons {
        home: png_menu_bmp(factory.as_ref(), include_bytes!("tray_icons/home.png")),
        admin: png_menu_bmp(factory.as_ref(), include_bytes!("tray_icons/settings.png")),
        stage: png_menu_bmp(factory.as_ref(), include_bytes!("tray_icons/artboard.png")),
        overlay: png_menu_bmp(factory.as_ref(), include_bytes!("tray_icons/device-tv.png")),
        copy: png_menu_bmp(factory.as_ref(), include_bytes!("tray_icons/copy.png")),
        about: png_menu_bmp(
            factory.as_ref(),
            include_bytes!("tray_icons/info-circle.png"),
        ),
        restart: png_menu_bmp(factory.as_ref(), include_bytes!("tray_icons/refresh.png")),
        quit: png_menu_bmp(factory.as_ref(), include_bytes!("tray_icons/x.png")),
    }
}

fn png_menu_bmp(factory: Option<&IWICImagingFactory>, png: &[u8]) -> HBITMAP {
    let Some(factory) = factory else {
        return HBITMAP::default();
    };
    unsafe { png_menu_bmp_inner(factory, png) }.unwrap_or_default()
}

unsafe fn png_menu_bmp_inner(
    factory: &IWICImagingFactory,
    png: &[u8],
) -> windows::core::Result<HBITMAP> {
    let stream = factory.CreateStream()?;
    stream.InitializeFromMemory(png)?;
    let decoder = factory.CreateDecoderFromStream(
        &stream,
        std::ptr::null(),
        WICDecodeMetadataCacheOnDemand,
    )?;
    let frame = decoder.GetFrame(0)?;
    let converter = factory.CreateFormatConverter()?;
    converter.Initialize(
        &frame,
        &GUID_WICPixelFormat32bppBGRA,
        WICBitmapDitherTypeNone,
        None::<&IWICPalette>,
        0.0,
        WICBitmapPaletteTypeCustom,
    )?;
    let size = GetSystemMetrics(SM_CXSMICON).clamp(16, 64) as u32;
    let scaler = factory.CreateBitmapScaler()?;
    scaler.Initialize(&converter, size, size, WICBitmapInterpolationModeFant)?;
    let stride = size * 4;
    let mut pixels = vec![0u8; (stride * size) as usize];
    scaler.CopyPixels(std::ptr::null(), stride, &mut pixels)?;

    let color = GetSysColor(COLOR_MENUTEXT);
    let (tr, tg, tb) = (
        (color & 0xFF) as u8,
        ((color >> 8) & 0xFF) as u8,
        ((color >> 16) & 0xFF) as u8,
    );
    for px in pixels.chunks_exact_mut(4) {
        let a = px[3] as u16;
        px[0] = ((tb as u16 * a) / 255) as u8;
        px[1] = ((tg as u16 * a) / 255) as u8;
        px[2] = ((tr as u16 * a) / 255) as u8;
    }

    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: size as i32,
            biHeight: -(size as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
    let hbmp = CreateDIBSection(
        HDC::default(),
        &bmi,
        DIB_RGB_COLORS,
        &mut bits,
        HANDLE::default(),
        0,
    )?;
    if !bits.is_null() {
        std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits.cast::<u8>(), pixels.len());
    }
    Ok(hbmp)
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

fn copy_text(hwnd: HWND, text: &str) {
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes = wide.len() * size_of::<u16>();
    unsafe {
        // Another process can hold the clipboard for a few ms.
        let mut opened = OpenClipboard(hwnd).is_ok();
        if !opened {
            for _ in 0..8 {
                std::thread::sleep(std::time::Duration::from_millis(10));
                if OpenClipboard(hwnd).is_ok() {
                    opened = true;
                    break;
                }
            }
        }
        if !opened {
            return;
        }
        let _ = EmptyClipboard();
        let Ok(hmem) = GlobalAlloc(GMEM_MOVEABLE, bytes) else {
            let _ = CloseClipboard();
            return;
        };
        let ptr = GlobalLock(hmem);
        if ptr.is_null() {
            let _ = GlobalFree(hmem);
            let _ = CloseClipboard();
            return;
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr.cast::<u16>(), wide.len());
        let _ = GlobalUnlock(hmem);
        // 13 = CF_UNICODETEXT
        if SetClipboardData(13, HANDLE(hmem.0)).is_err() {
            let _ = GlobalFree(hmem);
            let _ = CloseClipboard();
            return;
        }
        let _ = CloseClipboard();
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
