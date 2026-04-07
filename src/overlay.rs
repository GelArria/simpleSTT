use log::info;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows::Win32::UI::WindowsAndMessaging::*;

const TIMER_ID: usize = 1;
const ID_SETTINGS: usize = 1001;
const ID_QUIT: usize = 1002;

#[derive(Clone, Copy, PartialEq)]
pub enum OverlayState {
    Idle,
    Recording,
}

struct OverlayInner {
    state: OverlayState,
    size: u32,
    dragging: bool,
    drag_offset: (i32, i32),
    click_pos: (i32, i32),
    pulse_phase: f64,
    hover: bool,
}

unsafe impl Send for OverlayInner {}

static INNER: OnceLock<Mutex<OverlayInner>> = OnceLock::new();
static RECORDING_FLAG: OnceLock<Arc<AtomicBool>> = OnceLock::new();
static OVERLAY_HWND: AtomicIsize = AtomicIsize::new(0);

fn get_inner() -> &'static Mutex<OverlayInner> {
    INNER.get_or_init(|| {
        Mutex::new(OverlayInner {
            state: OverlayState::Idle,
            size: 48,
            dragging: false,
            drag_offset: (0, 0),
            click_pos: (0, 0),
            pulse_phase: 0.0,
            hover: false,
        })
    })
}

fn sync_recording_flag(state: OverlayState) {
    if let Some(flag) = RECORDING_FLAG.get() {
        flag.store(state == OverlayState::Recording, Ordering::SeqCst);
    }
}

fn set_state(state: OverlayState) {
    {
        let mut guard = get_inner().lock().unwrap();
        guard.state = state;
    }
    sync_recording_flag(state);
    let hwnd_raw = OVERLAY_HWND.load(Ordering::SeqCst);
    if hwnd_raw != 0 {
        let hwnd = HWND(hwnd_raw as *mut _);
        unsafe {
            let _ = InvalidateRect(hwnd, None, false);
        }
    }
}

fn toggle_overlay_state() {
    let state = {
        let mut guard = get_inner().lock().unwrap();
        guard.state = if guard.state == OverlayState::Idle {
            OverlayState::Recording
        } else {
            OverlayState::Idle
        };
        guard.state
    };
    sync_recording_flag(state);
    let hwnd_raw = OVERLAY_HWND.load(Ordering::SeqCst);
    if hwnd_raw != 0 {
        let hwnd = HWND(hwnd_raw as *mut _);
        unsafe {
            let _ = InvalidateRect(hwnd, None, false);
        }
    }
    info!(
        "recording {}",
        if state == OverlayState::Recording {
            "enabled"
        } else {
            "disabled"
        }
    );
}

fn make_colorref(r: i32, g: i32, b: i32) -> COLORREF {
    COLORREF(
        (b.max(0).min(255) as u32) << 16
            | (g.max(0).min(255) as u32) << 8
            | r.max(0).min(255) as u32,
    )
}

fn draw_filled_rounded_rect(hdc: HDC, x: i32, y: i32, w: i32, h: i32, radius: i32, brush: HBRUSH) {
    unsafe {
        let rgn = CreateRoundRectRgn(x, y, x + w, y + h, radius, radius);
        let old = SelectObject(hdc, brush);
        let _ = PaintRgn(hdc, rgn);
        let _ = SelectObject(hdc, old);
        let _ = DeleteObject(rgn);
    }
}

fn draw_rounded_border(hdc: HDC, x: i32, y: i32, w: i32, h: i32, radius: i32, brush: HBRUSH) {
    unsafe {
        let rgn = CreateRoundRectRgn(x, y, x + w, y + h, radius, radius);
        let _ = FrameRgn(hdc, rgn, brush, 2, 2);
        let _ = DeleteObject(rgn);
    }
}

pub fn create_overlay(opacity: u8, size: u32, recording: Arc<AtomicBool>) -> Result<(), String> {
    {
        let mut guard = get_inner().lock().unwrap();
        guard.size = if size > 0 { size } else { 48 };
    }
    let _ = RECORDING_FLAG.set(recording.clone());
    set_state(OverlayState::Idle);
    recording.store(false, Ordering::SeqCst);

    unsafe {
        let instance = match GetModuleHandleW(None) {
            Ok(h) => h,
            Err(e) => return Err(format!("GetModuleHandleW failed: {}", e)),
        };

        let class_name = windows::core::w!("SimpleSTTOverlay");

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(overlay_wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance.into(),
            hIcon: HICON::default(),
            hCursor: match LoadCursorW(None, IDC_HAND) {
                Ok(c) => c,
                Err(_) => HCURSOR::default(),
            },
            hbrBackground: HBRUSH::default(),
            lpszMenuName: windows::core::PCWSTR::null(),
            lpszClassName: class_name,
            hIconSm: HICON::default(),
        };

        let _ = RegisterClassExW(&wc);

        let actual_size = get_inner().lock().unwrap().size as i32;
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let x = screen_w - actual_size - 20;
        let y = 20;

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class_name,
            windows::core::w!(""),
            WS_POPUP,
            x,
            y,
            actual_size,
            actual_size,
            HWND::default(),
            HMENU::default(),
            instance,
            None,
        )
        .map_err(|e| format!("CreateWindowExW failed: {}", e))?;

        OVERLAY_HWND.store(hwnd.0 as isize, Ordering::SeqCst);

        SetLayeredWindowAttributes(hwnd, COLORREF(0), opacity, LWA_ALPHA)
            .map_err(|e| format!("SetLayeredWindowAttributes failed: {}", e))?;

        let rgn = CreateRoundRectRgn(0, 0, actual_size + 1, actual_size + 1, 14, 14);
        SetWindowRgn(hwnd, rgn, true);

        let _ = SetTimer(hwnd, TIMER_ID, 50, None);

        let _ = ShowWindow(hwnd, SW_SHOWNA);
    }

    Ok(())
}

fn draw_emoji(hdc: HDC, emoji: &str, cx: i32, cy: i32, size: i32) {
    unsafe {
        let font = CreateFontW(
            size,
            0,
            0,
            0,
            FW_NORMAL.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET.0 as u32,
            0,
            0,
            CLEARTYPE_QUALITY.0 as u32,
            0,
            windows::core::w!("Segoe UI Emoji"),
        );
        let old_font = SelectObject(hdc, font);

        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, COLORREF(0x00_FFFFFF));

        let mut wide: Vec<u16> = emoji.encode_utf16().chain(std::iter::once(0)).collect();
        let mut rect = RECT {
            left: cx - size,
            top: cy - size,
            right: cx + size,
            bottom: cy + size,
        };
        let _ = DrawTextExW(
            hdc,
            &mut wide,
            &mut rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            None,
        );

        let _ = SelectObject(hdc, old_font);
        let _ = DeleteObject(font);
    }
}

extern "system" fn overlay_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_TIMER => {
                if let Ok(mut guard) = get_inner().lock() {
                    guard.pulse_phase += 0.12;
                }
                let _ = InvalidateRect(hwnd, None, false);
                LRESULT(0)
            }

            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);

                let (size_i32, state_val, phase, is_hover) = {
                    let guard = get_inner().lock().unwrap();
                    (
                        guard.size as i32,
                        guard.state,
                        guard.pulse_phase,
                        guard.hover,
                    )
                };

                let m = 3;
                let inner = size_i32 - m * 2;
                let cx = size_i32 / 2;
                let cy = size_i32 / 2;

                if state_val == OverlayState::Recording {
                    let pulse = phase.sin();
                    let t = (0.5 + 0.5 * pulse) as f64;

                    let glow_size = inner + 8;
                    let glow_color = make_colorref((40.0 + 40.0 * t) as i32, 10, 10);
                    let glow_brush = CreateSolidBrush(glow_color);
                    draw_filled_rounded_rect(
                        hdc,
                        m - 4,
                        m - 4,
                        glow_size + 8,
                        glow_size + 8,
                        16,
                        glow_brush,
                    );
                    let _ = DeleteObject(glow_brush);

                    let bg_color = make_colorref((170.0 + 50.0 * t) as i32, 35, 35);
                    let bg_brush = CreateSolidBrush(bg_color);
                    draw_filled_rounded_rect(hdc, m, m, inner, inner, 12, bg_brush);
                    let _ = DeleteObject(bg_brush);

                    let border_color = make_colorref((220.0 + 35.0 * t) as i32, 80, 80);
                    let border_brush = CreateSolidBrush(border_color);
                    draw_rounded_border(hdc, m, m, inner, inner, 12, border_brush);
                    let _ = DeleteObject(border_brush);

                    draw_emoji(hdc, "\u{1F3A4}", cx, cy - 1, 22);
                } else {
                    let base = if is_hover { 85 } else { 55 };
                    let bg_color = make_colorref(base, base, base + 8);
                    let bg_brush = CreateSolidBrush(bg_color);
                    draw_filled_rounded_rect(hdc, m, m, inner, inner, 12, bg_brush);
                    let _ = DeleteObject(bg_brush);

                    let border_val = if is_hover { 110 } else { 75 };
                    let border_brush =
                        CreateSolidBrush(make_colorref(border_val, border_val, border_val + 5));
                    draw_rounded_border(hdc, m, m, inner, inner, 12, border_brush);
                    let _ = DeleteObject(border_brush);

                    draw_emoji(hdc, "\u{1F3A4}", cx, cy - 1, 22);
                }

                let _ = EndPaint(hwnd, &ps);
                LRESULT(0)
            }

            WM_LBUTTONDOWN => {
                let mut pt = POINT { x: 0, y: 0 };
                let _ = GetCursorPos(&mut pt);
                let mut rect = RECT::default();
                let _ = GetWindowRect(hwnd, &mut rect);
                if let Ok(mut guard) = get_inner().lock() {
                    guard.click_pos = (pt.x, pt.y);
                    guard.drag_offset = (pt.x - rect.left, pt.y - rect.top);
                    guard.dragging = true;
                }
                let _ = SetCapture(hwnd);
                LRESULT(0)
            }

            WM_LBUTTONUP => {
                let mut pt = POINT { x: 0, y: 0 };
                let _ = GetCursorPos(&mut pt);
                let (dx, dy) = {
                    let guard = get_inner().lock().unwrap();
                    (
                        (pt.x - guard.click_pos.0).abs(),
                        (pt.y - guard.click_pos.1).abs(),
                    )
                };

                if dx < 5 && dy < 5 {
                    toggle_overlay_state();
                }

                if let Ok(mut guard) = get_inner().lock() {
                    guard.dragging = false;
                }

                let _ = ReleaseCapture();
                LRESULT(0)
            }

            WM_MOUSEMOVE => {
                {
                    let mut guard = get_inner().lock().unwrap();
                    if !guard.dragging {
                        guard.hover = true;
                    }
                }
                let is_dragging = {
                    let guard = get_inner().lock().unwrap();
                    guard.dragging
                };

                if is_dragging {
                    let mut pt = POINT { x: 0, y: 0 };
                    let _ = GetCursorPos(&mut pt);
                    let offset = {
                        let guard = get_inner().lock().unwrap();
                        guard.drag_offset
                    };
                    let _ = SetWindowPos(
                        hwnd,
                        HWND_TOPMOST,
                        pt.x - offset.0,
                        pt.y - offset.1,
                        0,
                        0,
                        SWP_NOSIZE | SWP_NOZORDER,
                    );
                }
                LRESULT(0)
            }

            WM_RBUTTONUP => {
                let mut pt = POINT { x: 0, y: 0 };
                let _ = GetCursorPos(&mut pt);
                let hmenu = match CreatePopupMenu() {
                    Ok(m) => m,
                    Err(_) => return LRESULT(0),
                };
                let _ = AppendMenuW(
                    hmenu,
                    MF_STRING,
                    ID_SETTINGS,
                    windows::core::w!("Settings..."),
                );
                let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, windows::core::PCWSTR::null());
                let _ = AppendMenuW(hmenu, MF_STRING, ID_QUIT, windows::core::w!("Quit"));
                let _ = TrackPopupMenu(hmenu, TPM_RIGHTBUTTON, pt.x, pt.y, 0, hwnd, None);
                let _ = DestroyMenu(hmenu);
                LRESULT(0)
            }

            WM_COMMAND => {
                let id = (wparam.0 as usize) & 0xFFFF;
                if id == ID_QUIT {
                    PostQuitMessage(0);
                } else if id == ID_SETTINGS {
                    info!("settings clicked (stub)");
                }
                LRESULT(0)
            }

            WM_DESTROY => {
                let _ = KillTimer(hwnd, TIMER_ID);
                OVERLAY_HWND.store(0, Ordering::SeqCst);
                PostQuitMessage(0);
                LRESULT(0)
            }

            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

pub fn run_message_loop() -> i32 {
    let mut msg = MSG::default();
    unsafe {
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            if msg.message == WM_HOTKEY {
                toggle_overlay_state();
                continue;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    msg.wParam.0 as i32
}
