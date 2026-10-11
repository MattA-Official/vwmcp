//! How the server gets Vectorworks to run its pump.
//!
//! On Windows this posts the menu command's keyboard shortcut into the
//! Vectorworks message queue (works while VW is in the background) and
//! clears VW's own error dialogs so an unattended run never stalls. On other
//! platforms there is no trigger; tests drive the pump directly.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[cfg_attr(not(windows), allow(dead_code))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Fire {
    /// The shortcut was delivered.
    Sent { foreground: bool },
    /// A Vectorworks dialog is open; the shortcut would land in it.
    DialogOpen { titles: Vec<String> },
    /// No Vectorworks window was found.
    NotRunning,
    /// No trigger on this platform (or delivery failed).
    Unavailable { reason: String },
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct VwStatus {
    pub running: bool,
    pub minimised: bool,
    pub pid: Option<u32>,
    pub window_title: Option<String>,
    pub foreground: bool,
    pub open_dialogs: Vec<String>,
}

pub trait Trigger: Send + Sync {
    fn fire(&self) -> Fire;
    /// Close Vectorworks error dialogs (content-matched). Returns how many.
    fn dismiss_error_dialogs(&self) -> usize;
    fn status(&self) -> VwStatus;
    /// PNG of the Vectorworks main window.
    fn screenshot(&self, max_width: u32) -> anyhow::Result<Vec<u8>>;
    /// Called a few seconds after each job: undo anything `fire` changed
    /// (re-minimise a window it had to restore) once no more jobs arrive.
    fn settle(&self) {}
}

#[cfg_attr(windows, allow(dead_code))]
pub struct NoTrigger;

impl Trigger for NoTrigger {
    fn fire(&self) -> Fire {
        Fire::Unavailable {
            reason: "the Vectorworks trigger only exists on Windows".into(),
        }
    }
    fn dismiss_error_dialogs(&self) -> usize {
        0
    }
    fn status(&self) -> VwStatus {
        VwStatus::default()
    }
    fn screenshot(&self, _: u32) -> anyhow::Result<Vec<u8>> {
        anyhow::bail!("screenshots are only supported on Windows")
    }
}

/// Keyboard shortcut assigned to the menu command, e.g. `ctrl+shift+b`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hotkey {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    /// Windows virtual-key code of the main key.
    pub vk: u16,
}

impl Hotkey {
    pub fn parse(s: &str) -> Result<Self, String> {
        let mut hk = Hotkey {
            ctrl: false,
            shift: false,
            alt: false,
            vk: 0,
        };
        for part in s.split('+').map(|p| p.trim().to_ascii_lowercase()) {
            match part.as_str() {
                "ctrl" | "control" | "strg" => hk.ctrl = true,
                "shift" | "umschalt" => hk.shift = true,
                "alt" => hk.alt = true,
                k if k.len() == 1 && k.chars().all(|c| c.is_ascii_alphanumeric()) => {
                    hk.vk = k.to_ascii_uppercase().as_bytes()[0] as u16;
                }
                k if k.starts_with('f')
                    && k[1..].parse::<u16>().is_ok_and(|n| (1..=24).contains(&n)) =>
                {
                    hk.vk = 0x70 + k[1..].parse::<u16>().unwrap() - 1; // VK_F1..
                }
                other => return Err(format!("unknown key '{other}' in hotkey '{s}'")),
            }
        }
        if hk.vk == 0 {
            return Err(format!("hotkey '{s}' has no main key"));
        }
        if !(hk.ctrl || hk.alt) {
            return Err(format!(
                "hotkey '{s}' needs Ctrl or Alt so it cannot be typed by accident"
            ));
        }
        Ok(hk)
    }

    pub fn from_env() -> Result<Self, String> {
        Self::parse(&std::env::var("VWMCP_HOTKEY").unwrap_or_else(|_| "ctrl+shift+b".into()))
    }
}

impl std::fmt::Display for Hotkey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.ctrl {
            f.write_str("Ctrl+")?;
        }
        if self.alt {
            f.write_str("Alt+")?;
        }
        if self.shift {
            f.write_str("Shift+")?;
        }
        if (0x70..=0x87).contains(&self.vk) {
            write!(f, "F{}", self.vk - 0x70 + 1)
        } else {
            write!(f, "{}", self.vk as u8 as char)
        }
    }
}

pub fn platform_trigger(hotkey: Hotkey) -> Box<dyn Trigger> {
    #[cfg(windows)]
    {
        Box::new(crate::win::WinTrigger::new(hotkey))
    }
    #[cfg(not(windows))]
    {
        let _ = hotkey;
        Box::new(NoTrigger)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hotkeys() {
        let h = Hotkey::parse("Ctrl+Shift+B").unwrap();
        assert_eq!(
            h,
            Hotkey {
                ctrl: true,
                shift: true,
                alt: false,
                vk: b'B' as u16
            }
        );
        assert_eq!(h.to_string(), "Ctrl+Shift+B");
        assert_eq!(Hotkey::parse("ctrl+alt+f9").unwrap().vk, 0x78);
        assert!(Hotkey::parse("shift+b").is_err());
        assert!(Hotkey::parse("ctrl+shift").is_err());
        assert!(Hotkey::parse("ctrl+hyper+b").is_err());
    }
}
