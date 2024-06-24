// src/lib.rs
#![deny(clippy::all)]

use napi_derive::napi;
use napi::{JsFunction, Result};
use serde_json::{self, json};
use serde::Serialize;
use std::thread::spawn;
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage, MSG,
    WH_MOUSE_LL, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_RBUTTONDOWN, WM_RBUTTONUP,
    MSLLHOOKSTRUCT,
};
use napi::threadsafe_function::{ErrorStrategy, ThreadsafeFunction, ThreadsafeFunctionCallMode};

#[derive(Serialize)]
struct MouseEvent {
    event_type: String,
    x: i32,
    y: i32,
}

unsafe extern "system" fn mouse_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 {
        let mouse_info = &*(lparam.0 as *const MSLLHOOKSTRUCT);
        let event = match wparam.0 as u32 {
            WM_MOUSEMOVE => MouseEvent {
                event_type: "MouseMove".to_string(),
                x: mouse_info.pt.x,
                y: mouse_info.pt.y,
            },
            WM_LBUTTONDOWN => MouseEvent {
                event_type: "ButtonPress".to_string(),
                x: mouse_info.pt.x,
                y: mouse_info.pt.y,
            },
            WM_LBUTTONUP => MouseEvent {
                event_type: "ButtonRelease".to_string(),
                x: mouse_info.pt.x,
                y: mouse_info.pt.y,
            },
            WM_RBUTTONDOWN => MouseEvent {
                event_type: "ButtonPress".to_string(),
                x: mouse_info.pt.x,
                y: mouse_info.pt.y,
            },
            WM_RBUTTONUP => MouseEvent {
                event_type: "ButtonRelease".to_string(),
                x: mouse_info.pt.x,
                y: mouse_info.pt.y,
            },
            _ => return CallNextHookEx(None, code, wparam, lparam),
        };

        if let Some(jsfn) = &*JSFN.lock().unwrap() {
            let event_json = serde_json::to_string(&event).unwrap();
            jsfn.call(event_json, ThreadsafeFunctionCallMode::NonBlocking);
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

lazy_static::lazy_static! {
    static ref JSFN: std::sync::Mutex<Option<ThreadsafeFunction<String, ErrorStrategy::Fatal>>> = std::sync::Mutex::new(None);
}

#[napi(ts_args_type = "callback: (event: string) => void")]
pub fn start_listener(callback: JsFunction) -> Result<()> {
    let jsfn: ThreadsafeFunction<String, ErrorStrategy::Fatal> =
        callback.create_threadsafe_function(0, |ctx| Ok(vec![ctx.value]))?;

    *JSFN.lock().unwrap() = Some(jsfn);

    spawn(move || {
        unsafe {
            let hook = SetWindowsHookExW(
                WH_MOUSE_LL,
                Some(mouse_proc),
                HINSTANCE(0),
                0,
            );
            let mut msg: MSG = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    });

    Ok(())
}
