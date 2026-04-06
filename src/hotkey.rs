use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN, MOD_NOREPEAT,
    HOT_KEY_MODIFIERS,
};
use log::{error, info, warn};

const VK_F1: u32 = 0x70;
const VK_SPACE: u32 = 0x20;
const VK_ESCAPE: u32 = 0x1B;
const VK_RETURN: u32 = 0x0D;
const VK_TAB: u32 = 0x09;
const VK_BACK: u32 = 0x08;
const VK_DELETE: u32 = 0x2E;

fn parse_modifier(s: &str) -> Option<HOT_KEY_MODIFIERS> {
    match s {
        "Ctrl" | "Control" => Some(MOD_CONTROL),
        "Alt" => Some(MOD_ALT),
        "Shift" => Some(MOD_SHIFT),
        "Win" | "Super" => Some(MOD_WIN),
        _ => None,
    }
}

fn parse_vk(s: &str) -> Option<u32> {
    let upper = s.to_uppercase();
    if upper.starts_with('F') {
        let num: u32 = upper[1..].parse().ok()?;
        if (1..=24).contains(&num) {
            return Some(VK_F1 + num - 1);
        }
        return None;
    }
    match upper.as_str() {
        "SPACE" => Some(VK_SPACE),
        "ESCAPE" | "ESC" => Some(VK_ESCAPE),
        "ENTER" | "RETURN" => Some(VK_RETURN),
        "TAB" => Some(VK_TAB),
        "BACKSPACE" => Some(VK_BACK),
        "DELETE" | "DEL" => Some(VK_DELETE),
        _ => {
            let bytes = upper.as_bytes();
            if bytes.len() == 1 && (bytes[0].is_ascii_uppercase() || bytes[0].is_ascii_digit()) {
                return Some(bytes[0] as u32);
            }
            None
        }
    }
}

pub struct GlobalHotkey {
    id: u32,
    registered: bool,
}

impl GlobalHotkey {
    pub fn register(key_str: &str, id: u32) -> Result<Self, String> {
        let parts: Vec<&str> = key_str.split('+').collect();
        if parts.is_empty() {
            return Err("empty hotkey string".into());
        }

        let mut modifiers = HOT_KEY_MODIFIERS(0);
        let key_part;

        if parts.len() == 1 {
            key_part = parts[0].trim();
        } else {
            key_part = parts.last().unwrap().trim();
            for part in &parts[..parts.len() - 1] {
                let trimmed = part.trim();
                if let Some(mod_flag) = parse_modifier(trimmed) {
                    modifiers = HOT_KEY_MODIFIERS(modifiers.0 | mod_flag.0);
                } else {
                    return Err(format!("unknown modifier: {}", trimmed));
                }
            }
        }

        let vk = parse_vk(key_part).ok_or_else(|| format!("unknown key: {}", key_part))?;
        modifiers = HOT_KEY_MODIFIERS(modifiers.0 | MOD_NOREPEAT.0);

        unsafe {
            match RegisterHotKey(HWND::default(), id as i32, modifiers, vk) {
                Ok(()) => {
                    info!("hotkey registered: {}", key_str);
                    Ok(Self { id, registered: true })
                }
                Err(e) => {
                    error!("RegisterHotKey failed: {}", e);
                    Err(format!("RegisterHotKey failed: {}", e))
                }
            }
        }
    }

    pub fn unregister(&mut self) {
        if self.registered {
            unsafe {
                if let Err(e) = UnregisterHotKey(HWND::default(), self.id as i32) {
                    warn!("UnregisterHotKey failed: {}", e);
                } else {
                    info!("hotkey unregistered: {}", self.id);
                }
            }
            self.registered = false;
        }
    }

}

impl Drop for GlobalHotkey {
    fn drop(&mut self) {
        self.unregister();
    }
}
