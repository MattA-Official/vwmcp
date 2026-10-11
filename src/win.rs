//! Windows trigger: delivers the menu command's shortcut to Vectorworks,
//! clears its error dialogs, and captures its window.
//!
//! Ported from the vwx-mcp native palette (bridge v13), but run from this
//! external process so no C++ plug-in has to be built or installed:
//!
//! * VW in the foreground: a real keystroke via `SendInput`.
//! * VW in the background: attach to VW's UI thread so we share its key
//!   state, mark Ctrl/Shift as held, and post WM_KEYDOWN/UP into VW's queue.
//!   VW's own `TranslateAccelerator` then fires the menu command. Other
//!   applications never see the keys.
//!
//! Nothing here executes anything inside Vectorworks; the worst case of a
//! failed trigger is a queued job that times out visibly.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleBitmap, CreateCompatibleDC,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, HGDIOBJ, ReleaseDC, SelectObject,
};
use windows::Win32::Storage::Xps::{PRINT_WINDOW_FLAGS, PrintWindow};
use windows::Win32::System::Threading::{
    AttachThreadInput, GetCurrentThreadId, OpenProcess, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyboardState, INPUT, INPUT_0, INPUT_KEYBOARD, IsWindowEnabled, KEYBD_EVENT_FLAGS,
    KEYBDINPUT, KEYEVENTF_KEYUP, MAPVK_VK_TO_VSC, MapVirtualKeyW, SendInput, SetKeyboardState,
    VIRTUAL_KEY, VK_CONTROL, VK_MENU, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BM_CLICK, EnumChildWindows, EnumWindows, GW_OWNER, GetClassNameW, GetForegroundWindow,
    GetWindow, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, HWND_BOTTOM, IDOK,
    IsIconic, IsWindowVisible, MSG, PM_NOREMOVE, PeekMessageW, PostMessageW, SMTO_ABORTIFHUNG,
    SW_SHOWMINNOACTIVE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SendMessageTimeoutW, SetWindowPos, ShowWindow, WM_CLOSE, WM_COMMAND, WM_KEYDOWN, WM_KEYUP,
};
use windows::core::{BOOL, PWSTR};

use crate::trigger::{Fire, Hotkey, Trigger, VwStatus};

/// Text that marks a dialog as a Vectorworks script/engine error (German and
/// English builds). Never match product names: "Marionette" once matched the
/// Marionette editor itself in vwx-mcp.
const ERROR_SIGNATURES: &[&str] = &[
    "Script-Fehler",
    "Traceback",
    "Script Error",
    "Handle variable is NIL",
    "Invalid number of parameters",
    "Beim Kompilieren",
    "Error Output",
    "error occurred while compiling",
    "error occurred while executing",
];
const CLOSE_BUTTONS: &[&str] = &["OK", "Schlie", "Close"];
/// Window titles of Vectorworks' script-error dialogs. Some builds draw these
/// with their own widgets rather than a standard Windows dialog, so they are
/// recognised by title as well as by window class.
const ERROR_TITLES: &[&str] = &[
    "Script Error",
    "Script-Fehler",
    "Skriptfehler",
    "Scriptfehler",
];

fn is_error_title(t: &str) -> bool {
    let t = t.trim();
    ERROR_TITLES.iter().any(|e| t.contains(e))
        || ["Error", "Errors", "Fehler"]
            .iter()
            .any(|e| t.eq_ignore_ascii_case(e))
}

pub struct WinTrigger {
    hotkey: Hotkey,
    is_vw: Mutex<HashMap<u32, bool>>,
    /// Set when `fire` restored a minimised Vectorworks; holds the time of
    /// the latest trigger so `settle` can minimise it again once idle.
    restored: Mutex<Option<Instant>>,
}

/// How long Vectorworks stays restored after the last job before vwmcp
/// minimises it again.
const REMINIMISE_AFTER: Duration = Duration::from_millis(2500);

struct VwWindows {
    pid: u32,
    main: HWND,
    main_title: String,
    main_enabled: bool,
    dialogs: Vec<(HWND, String)>,
}

impl WinTrigger {
    pub fn new(hotkey: Hotkey) -> Self {
        WinTrigger {
            hotkey,
            is_vw: Mutex::new(HashMap::new()),
            restored: Mutex::new(None),
        }
    }

    fn pid_is_vectorworks(&self, pid: u32) -> bool {
        let mut cache = self.is_vw.lock().unwrap();
        if let Some(v) = cache.get(&pid) {
            return *v;
        }
        let v = process_image(pid)
            .map(|p| {
                let file = p
                    .rsplit(['\\', '/'])
                    .next()
                    .unwrap_or("")
                    .to_ascii_lowercase();
                file.starts_with("vectorworks") && file.ends_with(".exe")
            })
            .unwrap_or(false);
        if cache.len() > 512 {
            cache.clear();
        }
        cache.insert(pid, v);
        v
    }

    fn find(&self) -> Option<VwWindows> {
        let mut tops: Vec<HWND> = Vec::new();
        unsafe {
            let _ = EnumWindows(Some(collect_hwnd), LPARAM(&mut tops as *mut _ as isize));
        }
        let mut best: Option<(i64, VwWindows)> = None;
        let mut dialogs_by_pid: HashMap<u32, Vec<(HWND, String)>> = HashMap::new();
        for h in tops {
            unsafe {
                if !IsWindowVisible(h).as_bool() {
                    continue;
                }
                let mut pid = 0u32;
                GetWindowThreadProcessId(h, Some(&mut pid));
                if pid == 0 || !self.pid_is_vectorworks(pid) {
                    continue;
                }
                let class = class_name(h);
                let title = window_text(h);
                if class == "#32770" || is_error_title(&title) {
                    dialogs_by_pid.entry(pid).or_default().push((h, title));
                    continue;
                }
                let owned = GetWindow(h, GW_OWNER).is_ok_and(|o| !o.is_invalid());
                if owned || title.is_empty() {
                    continue;
                }
                let mut r = RECT::default();
                let _ = GetWindowRect(h, &mut r);
                let area = (r.right - r.left) as i64 * (r.bottom - r.top) as i64;
                if best.as_ref().is_none_or(|(a, _)| area > *a) {
                    best = Some((
                        area,
                        VwWindows {
                            pid,
                            main: h,
                            main_title: title,
                            main_enabled: IsWindowEnabled(h).as_bool(),
                            dialogs: Vec::new(),
                        },
                    ));
                }
            }
        }
        let (_, mut vw) = best?;
        vw.dialogs = dialogs_by_pid.remove(&vw.pid).unwrap_or_default();
        Some(vw)
    }

    fn send_foreground(&self) -> bool {
        let mut inputs = Vec::new();
        let mods: Vec<VIRTUAL_KEY> = self.modifiers();
        for m in &mods {
            inputs.push(key_input(*m, false));
        }
        inputs.push(key_input(VIRTUAL_KEY(self.hotkey.vk), false));
        inputs.push(key_input(VIRTUAL_KEY(self.hotkey.vk), true));
        for m in mods.iter().rev() {
            inputs.push(key_input(*m, true));
        }
        let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
        sent as usize == inputs.len()
    }

    fn post_background(&self, vw: &VwWindows) -> Result<(), String> {
        unsafe {
            let vw_tid = GetWindowThreadProcessId(vw.main, None);
            let me = GetCurrentThreadId();
            // AttachThreadInput needs this thread to own a message queue.
            let mut msg = MSG::default();
            let _ = PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE);
            if !AttachThreadInput(me, vw_tid, true).as_bool() {
                return Err("could not attach to the Vectorworks UI thread".into());
            }
            let mut old = [0u8; 256];
            let result = (|| {
                GetKeyboardState(&mut old).map_err(|e| e.to_string())?;
                let mut held = old;
                for m in self.modifiers() {
                    held[m.0 as usize] |= 0x80;
                }
                SetKeyboardState(&held).map_err(|e| e.to_string())?;
                let scan = MapVirtualKeyW(self.hotkey.vk as u32, MAPVK_VK_TO_VSC) as isize;
                let vk = WPARAM(self.hotkey.vk as usize);
                PostMessageW(Some(vw.main), WM_KEYDOWN, vk, LPARAM(1 | (scan << 16)))
                    .map_err(|e| e.to_string())?;
                PostMessageW(
                    Some(vw.main),
                    WM_KEYUP,
                    vk,
                    LPARAM(1 | (scan << 16) | (1 << 30) | (1 << 31)),
                )
                .map_err(|e| e.to_string())?;
                // VW must translate the posted keys while the modifiers still
                // read as held; restoring immediately would race it.
                std::thread::sleep(Duration::from_millis(120));
                Ok(())
            })();
            let _ = SetKeyboardState(&old);
            let _ = AttachThreadInput(me, vw_tid, false);
            result
        }
    }

    fn modifiers(&self) -> Vec<VIRTUAL_KEY> {
        let mut m = Vec::new();
        if self.hotkey.ctrl {
            m.push(VK_CONTROL);
        }
        if self.hotkey.alt {
            m.push(VK_MENU);
        }
        if self.hotkey.shift {
            m.push(VK_SHIFT);
        }
        m
    }
}

impl Trigger for WinTrigger {
    fn fire(&self) -> Fire {
        let Some(vw) = self.find() else {
            return Fire::NotRunning;
        };
        if !vw.dialogs.is_empty() || !vw.main_enabled {
            let mut titles: Vec<String> = vw.dialogs.iter().map(|(_, t)| t.clone()).collect();
            if titles.is_empty() {
                titles.push("modal window".into());
            }
            return Fire::DialogOpen { titles };
        }
        // A minimised Vectorworks ignores its shortcuts. Restore it without
        // activating it and send it to the back, so it stays out of the way;
        // `settle` minimises it again when the jobs stop.
        let mut restored = self.restored.lock().unwrap();
        if unsafe { IsIconic(vw.main) }.as_bool() {
            unsafe {
                let _ = ShowWindow(vw.main, SW_SHOWNOACTIVATE);
                let _ = SetWindowPos(
                    vw.main,
                    Some(HWND_BOTTOM),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
                );
            }
            *restored = Some(Instant::now());
            tracing::info!("restored the minimised Vectorworks window behind other windows");
            // Give it a moment to come back before sending keys.
            std::thread::sleep(Duration::from_millis(250));
        } else if restored.is_some() {
            *restored = Some(Instant::now());
        }
        drop(restored);
        let fg = unsafe { GetForegroundWindow() };
        let mut fg_pid = 0u32;
        unsafe { GetWindowThreadProcessId(fg, Some(&mut fg_pid)) };
        if fg_pid == vw.pid && self.send_foreground() {
            return Fire::Sent { foreground: true };
        }
        match self.post_background(&vw) {
            Ok(()) => Fire::Sent { foreground: false },
            Err(reason) => Fire::Unavailable { reason },
        }
    }

    fn dismiss_error_dialogs(&self) -> usize {
        let Some(vw) = self.find() else { return 0 };
        let mut n = 0;
        for (dlg, title) in &vw.dialogs {
            let children = child_windows(*dlg);
            let texts: Vec<(HWND, String, String)> = children
                .into_iter()
                .map(|c| (c, class_name(c), window_text(c)))
                .collect();
            let is_error = is_error_title(title)
                || title.contains("Fehler")
                || title.contains("Error")
                || texts
                    .iter()
                    .any(|(_, _, t)| ERROR_SIGNATURES.iter().any(|s| t.contains(s)));
            if !is_error {
                continue;
            }
            let button = texts.iter().find(|(_, cls, t)| {
                cls.eq_ignore_ascii_case("Button") && CLOSE_BUTTONS.iter().any(|b| t.contains(b))
            });
            unsafe {
                let mut out = 0usize;
                match button {
                    Some((b, _, _)) => {
                        SendMessageTimeoutW(
                            *b,
                            BM_CLICK,
                            WPARAM(0),
                            LPARAM(0),
                            SMTO_ABORTIFHUNG,
                            500,
                            Some(&mut out),
                        );
                    }
                    None if class_name(*dlg) == "#32770" => {
                        SendMessageTimeoutW(
                            *dlg,
                            WM_COMMAND,
                            WPARAM(IDOK.0 as usize),
                            LPARAM(0),
                            SMTO_ABORTIFHUNG,
                            500,
                            Some(&mut out),
                        );
                    }
                    // A custom-drawn dialog has no button windows to click;
                    // closing it is the same as pressing OK on an error.
                    None => {
                        let _ = PostMessageW(Some(*dlg), WM_CLOSE, WPARAM(0), LPARAM(0));
                    }
                }
            }
            tracing::info!(dialog = %title, "dismissed a Vectorworks error dialog");
            n += 1;
        }
        n
    }

    fn status(&self) -> VwStatus {
        match self.find() {
            None => VwStatus::default(),
            Some(vw) => {
                let fg = unsafe { GetForegroundWindow() };
                let mut fg_pid = 0u32;
                unsafe { GetWindowThreadProcessId(fg, Some(&mut fg_pid)) };
                VwStatus {
                    running: true,
                    minimised: unsafe { IsIconic(vw.main) }.as_bool(),
                    pid: Some(vw.pid),
                    window_title: Some(vw.main_title),
                    foreground: fg_pid == vw.pid,
                    open_dialogs: vw.dialogs.into_iter().map(|(_, t)| t).collect(),
                }
            }
        }
    }

    fn settle(&self) {
        let mut restored = self.restored.lock().unwrap();
        let Some(at) = *restored else { return };
        if at.elapsed() < REMINIMISE_AFTER {
            return;
        }
        *restored = None;
        drop(restored);
        if let Some(vw) = self.find() {
            let fg = unsafe { GetForegroundWindow() };
            let mut fg_pid = 0u32;
            unsafe { GetWindowThreadProcessId(fg, Some(&mut fg_pid)) };
            // If the user has switched to Vectorworks meanwhile, leave it be.
            if fg_pid != vw.pid {
                unsafe {
                    let _ = ShowWindow(vw.main, SW_SHOWMINNOACTIVE);
                }
            }
        }
    }

    fn screenshot(&self, max_width: u32) -> anyhow::Result<Vec<u8>> {
        let vw = self
            .find()
            .ok_or_else(|| anyhow::anyhow!("Vectorworks is not running"))?;
        if unsafe { IsIconic(vw.main) }.as_bool() {
            anyhow::bail!("the Vectorworks window is minimised; restore it to take a screenshot");
        }
        let (w, h, bgra) = capture(vw.main)?;
        let (w, h, rgb) = downscale_bgra_to_rgb(&bgra, w, h, max_width);
        let mut png_bytes = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut png_bytes, w, h);
            enc.set_color(png::ColorType::Rgb);
            enc.set_depth(png::BitDepth::Eight);
            let mut writer = enc.write_header()?;
            writer.write_image_data(&rgb)?;
        }
        Ok(png_bytes)
    }
}

fn key_input(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

unsafe extern "system" fn collect_hwnd(h: HWND, lp: LPARAM) -> BOOL {
    let v = unsafe { &mut *(lp.0 as *mut Vec<HWND>) };
    v.push(h);
    BOOL(1)
}

fn child_windows(parent: HWND) -> Vec<HWND> {
    let mut v: Vec<HWND> = Vec::new();
    unsafe {
        let _ = EnumChildWindows(
            Some(parent),
            Some(collect_hwnd),
            LPARAM(&mut v as *mut _ as isize),
        );
    }
    v
}

fn class_name(h: HWND) -> String {
    let mut buf = [0u16; 128];
    let n = unsafe { GetClassNameW(h, &mut buf) };
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

fn window_text(h: HWND) -> String {
    let mut buf = [0u16; 1024];
    let n = unsafe { GetWindowTextW(h, &mut buf) };
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

fn process_image(pid: u32) -> Option<String> {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok =
            QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = CloseHandle(h);
        ok.ok()?;
        Some(String::from_utf16_lossy(&buf[..len as usize]))
    }
}

/// Capture a window (even when covered or in the background) as BGRA rows.
fn capture(hwnd: HWND) -> anyhow::Result<(u32, u32, Vec<u8>)> {
    unsafe {
        let mut r = RECT::default();
        GetWindowRect(hwnd, &mut r)?;
        let (w, h) = ((r.right - r.left).max(1), (r.bottom - r.top).max(1));
        let screen = GetDC(Some(hwnd));
        let mem = CreateCompatibleDC(Some(screen));
        let bmp = CreateCompatibleBitmap(screen, w, h);
        let old = SelectObject(mem, HGDIOBJ(bmp.0));
        // PW_RENDERFULLCONTENT (2) also captures GPU-rendered content.
        let printed = PrintWindow(hwnd, mem, PRINT_WINDOW_FLAGS(2)).as_bool();
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h, // top-down rows
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut buf = vec![0u8; (w * h * 4) as usize];
        SelectObject(mem, old);
        let lines = GetDIBits(
            mem,
            bmp,
            0,
            h as u32,
            Some(buf.as_mut_ptr().cast()),
            &mut info,
            DIB_RGB_COLORS,
        );
        let _ = DeleteObject(HGDIOBJ(bmp.0));
        let _ = DeleteDC(mem);
        ReleaseDC(Some(hwnd), screen);
        if !printed || lines == 0 {
            anyhow::bail!("could not capture the Vectorworks window");
        }
        Ok((w as u32, h as u32, buf))
    }
}

/// Box-filter downscale to `max_width` (if wider) and drop alpha.
pub(crate) fn downscale_bgra_to_rgb(
    bgra: &[u8],
    w: u32,
    h: u32,
    max_width: u32,
) -> (u32, u32, Vec<u8>) {
    let max_width = max_width.max(64);
    let factor = w.div_ceil(max_width).max(1);
    let (nw, nh) = (w / factor, h / factor);
    let mut out = Vec::with_capacity((nw * nh * 3) as usize);
    for y in 0..nh {
        for x in 0..nw {
            let (mut b, mut g, mut r) = (0u32, 0u32, 0u32);
            for dy in 0..factor {
                for dx in 0..factor {
                    let i = (((y * factor + dy) * w + x * factor + dx) * 4) as usize;
                    b += bgra[i] as u32;
                    g += bgra[i + 1] as u32;
                    r += bgra[i + 2] as u32;
                }
            }
            let n = factor * factor;
            out.extend_from_slice(&[(r / n) as u8, (g / n) as u8, (b / n) as u8]);
        }
    }
    (nw, nh, out)
}

/// Add (`add = true`) or remove `dir` from the current user's PATH, so
/// `vwmcp` works in new terminals. Returns whether anything changed.
pub fn set_user_path_entry(dir: &std::path::Path, add: bool) -> anyhow::Result<bool> {
    use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
    use windows::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_EXPAND_SZ, REG_VALUE_TYPE, RegCloseKey,
        RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{HWND_BROADCAST, WM_SETTINGCHANGE};
    use windows::core::w;

    let dir = dir.to_string_lossy().trim_end_matches('\\').to_string();
    unsafe {
        let mut key = HKEY::default();
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            w!("Environment"),
            None,
            KEY_READ | KEY_WRITE,
            &mut key,
        )
        .ok()?;
        let result = (|| -> anyhow::Result<bool> {
            let mut kind = REG_VALUE_TYPE::default();
            let mut len = 0u32;
            let rc = RegQueryValueExW(key, w!("Path"), None, Some(&mut kind), None, Some(&mut len));
            let current = if rc == ERROR_FILE_NOT_FOUND {
                String::new()
            } else {
                rc.ok()?;
                let mut buf = vec![0u16; (len as usize).div_ceil(2) + 1];
                RegQueryValueExW(
                    key,
                    w!("Path"),
                    None,
                    Some(&mut kind),
                    Some(buf.as_mut_ptr().cast()),
                    Some(&mut len),
                )
                .ok()?;
                let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
                String::from_utf16_lossy(&buf[..end])
            };
            let entries: Vec<&str> = current
                .split(';')
                .filter(|s| !s.trim().is_empty())
                .collect();
            let present = entries
                .iter()
                .any(|e| e.trim_end_matches('\\').eq_ignore_ascii_case(&dir));
            let new: Vec<&str> = match (add, present) {
                (true, false) => entries
                    .iter()
                    .copied()
                    .chain(std::iter::once(dir.as_str()))
                    .collect(),
                (false, true) => entries
                    .iter()
                    .copied()
                    .filter(|e| !e.trim_end_matches('\\').eq_ignore_ascii_case(&dir))
                    .collect(),
                _ => return Ok(false),
            };
            let value: Vec<u16> = new
                .join(";")
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let bytes = std::slice::from_raw_parts(value.as_ptr().cast::<u8>(), value.len() * 2);
            // Keep the original type (usually REG_EXPAND_SZ, so %VARS% still expand).
            let ty = if kind.0 == 0 { REG_EXPAND_SZ } else { kind };
            RegSetValueExW(key, w!("Path"), None, ty, Some(bytes)).ok()?;
            Ok(true)
        })();
        let _ = RegCloseKey(key);
        let changed = result?;
        if changed {
            let mut out = 0usize;
            SendMessageTimeoutW(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                WPARAM(0),
                LPARAM(w!("Environment").as_ptr() as isize),
                SMTO_ABORTIFHUNG,
                1000,
                Some(&mut out),
            );
        }
        Ok(changed)
    }
}

#[cfg(test)]
mod tests {
    use super::is_error_title;

    #[test]
    fn error_titles() {
        assert!(is_error_title("Script Errors"));
        assert!(is_error_title("Script-Fehler"));
        assert!(is_error_title("Error"));
        assert!(!is_error_title("Object Info"));
        assert!(!is_error_title("Error Handling Palette"));
    }
}
