use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_KEYBOARD, INPUT_0, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    VIRTUAL_KEY,
};
use log::{error, info};
use std::thread;
use std::time::Duration;

pub fn inject_text(text: &str) -> Result<(), String> {
    for ch in text.chars() {
        let scan = ch as u16;

        let inputs = [
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0),
                        wScan: scan,
                        dwFlags: KEYEVENTF_UNICODE,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0),
                        wScan: scan,
                        dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
        ];

        let inserted = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
        if inserted == 0 {
            error!("SendInput failed for char '{}'", ch);
        }
        thread::sleep(Duration::from_millis(1));
    }

    info!("injected {} chars", text.chars().count());
    Ok(())
}
