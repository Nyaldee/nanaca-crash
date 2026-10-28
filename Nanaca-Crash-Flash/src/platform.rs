//! Ce qui dépend du système : le dossier des sauvegardes (Documents\My Games
//! sous Windows, ~/.local/share sous Linux, ou à côté de l'exécutable en
//! version portable), le plein écran sans bordure.

use std::path::PathBuf;

#[cfg(windows)]
pub use windows::{remember_window, set_borderless};
#[cfg(not(windows))]
pub use unix::{remember_window, set_borderless};
#[cfg(windows)]
use windows::user_data;
#[cfg(not(windows))]
use unix::user_data;

/// Le dossier de l'exécutable.
pub fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe().ok().and_then(|e| e.parent().map(PathBuf::from))
}

/// Version portable : un portable.txt à côté de l'exécutable range les
/// sauvegardes dans le dossier « saves » voisin.
pub fn portable() -> Option<PathBuf> {
    exe_dir().filter(|d| d.join("portable.txt").is_file()).map(|d| d.join("saves"))
}

/// Le dossier des sauvegardes d'un jeu : celui de la version portable, sinon
/// Documents\My Games\<jeu> (Windows) ou ~/.local/share/<jeu> (Linux).
pub fn saves(game: &str) -> Option<PathBuf> {
    portable().or_else(|| {
        let base = user_data()?;
        Some(if cfg!(windows) { base.join("My Games") } else { base }.join(game))
    })
}

#[cfg(not(windows))]
mod unix {
    use std::path::PathBuf;

    /// Les données des jeux : $XDG_DATA_HOME (~/.local/share).
    pub fn user_data() -> Option<PathBuf> {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").filter(|h| !h.is_empty()).map(|h| PathBuf::from(h).join(".local/share")))
    }

    /// Le bureau rend lui-même sa taille à la fenêtre.
    pub fn remember_window() {}

    /// Plein écran sans bordure, géré par le bureau (_NET_WM_STATE_FULLSCREEN).
    pub fn set_borderless(on: bool) {
        miniquad::window::set_fullscreen(on);
    }
}

#[cfg(windows)]
mod windows {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStringExt;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    #[repr(C)]
    struct Guid(u32, u16, u16, [u8; 8]);
    /// FOLDERID_Documents.
    const DOCUMENTS: Guid = Guid(0xFDD3_9AD0, 0x238F, 0x46AF, [0xAD, 0xB4, 0x6C, 0x85, 0x48, 0x03, 0x69, 0xC7]);

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    #[derive(Default)]
    struct MonitorInfo {
        size: u32,
        monitor: Rect,
        work: Rect,
        flags: u32,
    }

    #[link(name = "shell32")]
    unsafe extern "system" {
        fn SHGetKnownFolderPath(id: *const Guid, flags: u32, token: *mut c_void, path: *mut *mut u16) -> i32;
    }

    #[link(name = "ole32")]
    unsafe extern "system" {
        fn CoTaskMemFree(p: *mut c_void);
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn EnumThreadWindows(thread: u32, f: unsafe extern "system" fn(*mut c_void, isize) -> i32, param: isize) -> i32;
        fn GetClassNameW(wnd: *mut c_void, name: *mut u16, len: i32) -> i32;
        fn GetWindowRect(wnd: *mut c_void, rect: *mut Rect) -> i32;
        fn SetWindowLongPtrW(wnd: *mut c_void, index: i32, value: isize) -> isize;
        fn SetWindowPos(wnd: *mut c_void, after: *mut c_void, x: i32, y: i32, w: i32, h: i32, flags: u32) -> i32;
        fn MonitorFromWindow(wnd: *mut c_void, flags: u32) -> *mut c_void;
        fn GetMonitorInfoW(monitor: *mut c_void, info: *mut MonitorInfo) -> i32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentThreadId() -> u32;
    }

    const GWL_STYLE: i32 = -16;
    const WS_POPUP: isize = 0x8000_0000;
    const WS_VISIBLE: isize = 0x1000_0000;
    const WS_SYSMENU: isize = 0x0008_0000;
    /// Une fenêtre normale (titre, bords, réduire, agrandir).
    const WS_WINDOW: isize = 0x00CF_0000 | 0x0400_0000 | 0x0200_0000;
    const MONITOR_DEFAULTTONEAREST: u32 = 2;
    const SWP_FRAMECHANGED: u32 = 0x0020;
    const SWP_SHOWWINDOW: u32 = 0x0040;
    /// Classe de la fenêtre que crée miniquad.
    const WINDOW_CLASS: &str = "MINIQUADAPP";

    /// Le dossier Documents de l'utilisateur, tel que Windows le connaît
    /// (déplacé ou synchronisé compris).
    pub fn user_data() -> Option<PathBuf> {
        let mut path: *mut u16 = std::ptr::null_mut();
        // SAFETY : Windows écrit un chemin terminé par 0, libéré par
        // CoTaskMemFree après la copie.
        unsafe {
            let ok = SHGetKnownFolderPath(&DOCUMENTS, 0, std::ptr::null_mut(), &mut path) >= 0 && !path.is_null();
            let result = ok.then(|| {
                let len = (0..).take_while(|&i| *path.add(i) != 0).count();
                PathBuf::from(std::ffi::OsString::from_wide(std::slice::from_raw_parts(path, len)))
            });
            CoTaskMemFree(path.cast());
            result
        }
    }

    /// La fenêtre du jeu : celle de miniquad, sur le fil appelant.
    fn game_window() -> Option<*mut c_void> {
        unsafe extern "system" fn found(wnd: *mut c_void, param: isize) -> i32 {
            let mut name = [0u16; 32];
            // SAFETY : la taille du tampon est donnée ; `param` est l'adresse
            // de `out` ci-dessous, vivante pendant l'appel.
            let n = unsafe { GetClassNameW(wnd, name.as_mut_ptr(), name.len() as i32) };
            if String::from_utf16_lossy(&name[..n.max(0) as usize]) == WINDOW_CLASS {
                unsafe { *(param as *mut *mut c_void) = wnd };
                return 0;
            }
            1
        }
        let mut out: *mut c_void = std::ptr::null_mut();
        // SAFETY : rappel appelé pendant l'énumération seulement.
        unsafe { EnumThreadWindows(GetCurrentThreadId(), found, &mut out as *mut _ as isize) };
        (!out.is_null()).then_some(out)
    }

    /// Taille de la fenêtre au lancement (bords compris), retrouvée en
    /// revenant en fenêtre.
    static WINDOW_SIZE: OnceLock<(i32, i32)> = OnceLock::new();

    /// Retient la taille de la fenêtre telle qu'elle s'ouvre, avant tout
    /// redimensionnement.
    pub fn remember_window() {
        let Some(wnd) = game_window() else { return };
        let mut r = Rect::default();
        // SAFETY : `wnd` est la fenêtre du jeu, `r` un RECT.
        if unsafe { GetWindowRect(wnd, &mut r) } != 0 {
            let _ = WINDOW_SIZE.set((r.right - r.left, r.bottom - r.top));
        }
    }

    /// Plein écran sans bordure (l'écran de la fenêtre, dépassé d'un pixel
    /// en bas : Windows garde la main sur l'affichage), ou fenêtre à sa
    /// taille de départ, centrée.
    pub fn set_borderless(on: bool) {
        remember_window();
        let Some(wnd) = game_window() else { return };
        // SAFETY : `wnd` est la fenêtre du jeu ; les structures sont celles
        // de Windows, `size` rempli.
        unsafe {
            let mut info = MonitorInfo { size: size_of::<MonitorInfo>() as u32, ..MonitorInfo::default() };
            if GetMonitorInfoW(MonitorFromWindow(wnd, MONITOR_DEFAULTTONEAREST), &mut info) == 0 {
                return;
            }
            let m = info.monitor;
            let (style, x, y, w, h) = if on {
                (WS_POPUP | WS_VISIBLE | WS_SYSMENU, m.left, m.top, m.right - m.left, m.bottom - m.top + 1)
            } else {
                let (w, h) = WINDOW_SIZE.get().copied().unwrap_or((716, 439));
                let a = info.work;
                (WS_WINDOW | WS_VISIBLE, a.left + (a.right - a.left - w) / 2, a.top + (a.bottom - a.top - h) / 2, w, h)
            };
            SetWindowLongPtrW(wnd, GWL_STYLE, style);
            SetWindowPos(wnd, std::ptr::null_mut(), x, y, w, h, SWP_FRAMECHANGED | SWP_SHOWWINDOW);
        }
    }
}
