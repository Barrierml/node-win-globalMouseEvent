#[path = "./get_browser_url.rs"]
mod get_browser_url;

#[path = "./window_process.rs"]
mod window_processs;

use serde::Serialize;
use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc, Mutex,
};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use windows::Win32::Foundation::{BOOL, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, EnumWindows, GetForegroundWindow, GetMessageW,
    IsWindowVisible, SetWindowsHookExW, UnhookWindowsHookEx, MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_RBUTTONDOWN, WM_RBUTTONUP,
};

#[derive(Serialize, PartialEq)]
pub enum MouseEventType {
    MouseMove,
    LeftClickDown,
    LeftClickUp,
    RightClickDown,
    RightClickUp,
    MouseWheel,
}

#[derive(Serialize)]
pub struct MouseEvent {
    pub event_type: MouseEventType,
    pub position: (i32, i32),
    pub timestamp: u64,
    pub process_name: String,
    pub url: Option<String>,
}

static mut MONITORED_HWND_LIST: Option<Arc<Mutex<Vec<HWND>>>> = None;
static mut TX: Option<Sender<MouseEvent>> = None;

// 常量浏览器进程名, 支持获取地址的浏览器
const BROWSER_PROCESS_NAME: [&str; 3] = ["chrome.exe", "firefox.exe", "msedge.exe"];

extern "system" fn mouse_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        unsafe {
            let mouse_struct = &*(lparam.0 as *const MSLLHOOKSTRUCT);
            let foreground_window = GetForegroundWindow();
            let monitored_hwnd_list = MONITORED_HWND_LIST.as_ref().unwrap().lock().unwrap();
            if monitored_hwnd_list.contains(&foreground_window) {
                let event_type = match wparam.0 as u32 {
                    WM_MOUSEMOVE => MouseEventType::MouseMove,
                    WM_LBUTTONDOWN => MouseEventType::LeftClickDown,
                    WM_LBUTTONUP => MouseEventType::LeftClickUp,
                    WM_RBUTTONDOWN => MouseEventType::RightClickDown,
                    WM_RBUTTONUP => MouseEventType::RightClickUp,
                    WM_MOUSEWHEEL => MouseEventType::MouseWheel,
                    _ => return CallNextHookEx(None, code, wparam, lparam),
                };
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                let position = (mouse_struct.pt.x, mouse_struct.pt.y);
                let event = MouseEvent {
                    event_type,
                    position,
                    timestamp,
                    process_name: get_hwnd_process_name(foreground_window),
                    url: None,
                };
                if let Some(tx) = &TX {
                    tx.send(event).unwrap();
                }
            }
        }
    }

    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

pub fn start_listening() -> Receiver<MouseEvent> {
    let (tx, rx) = channel();
    unsafe {
        TX = Some(tx.clone());
    }

    let monitored_hwnd_list = Arc::new(Mutex::new(Vec::new()));
    unsafe {
        MONITORED_HWND_LIST = Some(monitored_hwnd_list.clone());
    }

    thread::spawn(move || unsafe {
        let hook_handle =
            SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), HINSTANCE::default(), 0).unwrap();

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            DispatchMessageW(&msg);
        }

        UnhookWindowsHookEx(hook_handle);
    });

    rx
}

pub fn add_process_to_monitor(hwnd: HWND) {
    unsafe {
        if let Some(monitored_hwnd_list) = &MONITORED_HWND_LIST {
            monitored_hwnd_list.lock().unwrap().push(hwnd);
            println!("Added HWND {:?} to monitor list", hwnd);
        }
    }
}

pub fn remove_process_from_monitor(hwnd: HWND) {
    unsafe {
        if let Some(monitored_hwnd_list) = &MONITORED_HWND_LIST {
            let mut list = monitored_hwnd_list.lock().unwrap();
            if let Some(pos) = list.iter().position(|&x| x == hwnd) {
                list.remove(pos);
                println!("Removed HWND {:?} from monitor list", hwnd);
            }
        }
    }
}

fn get_all_visible_windows() -> Vec<HWND> {
    let mut windows = Vec::new();
    unsafe {
        EnumWindows(
            Some(enum_window_callback),
            LPARAM(&mut windows as *mut _ as isize),
        );
    }
    windows
}

extern "system" fn enum_window_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        if IsWindowVisible(hwnd).as_bool() {
            let windows = &mut *(lparam.0 as *mut Vec<HWND>);
            windows.push(hwnd);
        }
    }
    true.into()
}

pub fn is_window_visible(hwnd: HWND) -> bool {
    unsafe { IsWindowVisible(hwnd).as_bool() }
}

// 利用 windows api 获取句柄对应的进程名
pub fn get_hwnd_process_name(hwnd: HWND) -> String {
    let process = window_processs::get_process_info(hwnd);
    process.name
}

pub fn find_new_processes(current_processes: &[HWND]) -> Vec<HWND> {
    let all_windows = get_all_visible_windows();
    all_windows
        .into_iter()
        .filter(|hwnd| !current_processes.contains(hwnd))
        .collect()
}

pub fn monitor_visible_processes() {
    let current_processes = Arc::new(Mutex::new(get_all_visible_windows()));
    let monitored_hwnd_list = Arc::clone(&current_processes);
    // 统一添加到监控列表
    for hwnd in current_processes.lock().unwrap().iter() {
        add_process_to_monitor(*hwnd);
    }

    thread::spawn(move || loop {
        let new_processes = find_new_processes(&current_processes.lock().unwrap());
        {
            let mut monitored_list = monitored_hwnd_list.lock().unwrap();
            for hwnd in new_processes {
                add_process_to_monitor(hwnd);
                monitored_list.push(hwnd);
            }
            let current_list: Vec<HWND> = monitored_list.clone();
            for &hwnd in &current_list {
                if !is_window_visible(hwnd) {
                    remove_process_from_monitor(hwnd);
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    });
}

// Function to process mouse events
fn process_mouse_events(rx: Receiver<MouseEvent>, callback: Arc<Mutex<dyn Fn(MouseEvent) + Send>>) {
    thread::spawn(move || {
        for event in rx {
            let mut event = event;
            if BROWSER_PROCESS_NAME.contains(&event.process_name.as_str()) {
                if event.event_type == MouseEventType::LeftClickDown
                    || event.event_type == MouseEventType::RightClickDown
                {
                    let url_result =
                        get_browser_url::get_brower_url(unsafe { GetForegroundWindow() });
                    if let Ok(url) = url_result {
                        event.url = Some(url);
                    }
                }
            }
            let callback = callback.lock().unwrap();
            callback(event);
        }
    });
}

pub fn start(cb: Arc<Mutex<dyn Fn(MouseEvent) + Send>>) {
    let rx = start_listening();
    process_mouse_events(rx, cb.clone());
    monitor_visible_processes();
}
