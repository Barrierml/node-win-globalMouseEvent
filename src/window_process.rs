use std::collections::HashMap;
use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::ptr;
use windows::Win32::Foundation::{BOOL, HMODULE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::ProcessStatus::{K32EnumProcessModules, K32GetModuleBaseNameW};
use windows::Win32::System::Threading::OpenProcess;
use windows::Win32::System::Threading::{PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, EnumChildWindows, EnumWindows, GetClassNameW, GetWindowTextW,
    GetWindowThreadProcessId, IsWindowVisible, MSLLHOOKSTRUCT,
};

pub struct ProcessInfo {
    pub hwnd: HWND,
    pub pid: u32,
    pub name: String,
}

extern "system" fn enum_window_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }

    if pid != 0 && unsafe { IsWindowVisible(hwnd) }.as_bool() {
        let process_map = unsafe { &mut *(lparam.0 as *mut HashMap<u32, ProcessInfo>) };

        let process_handle =
            unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid) };
        if let Ok(handle) = process_handle {
            let mut module: [HMODULE; 1] = [HMODULE::default(); 1];
            let mut cb_needed = 0;
            if unsafe {
                K32EnumProcessModules(
                    handle,
                    module.as_mut_ptr(),
                    std::mem::size_of_val(&module) as u32,
                    &mut cb_needed,
                )
            }
            .as_bool()
            {
                let mut module_name = vec![0u16; 1024];
                if unsafe { K32GetModuleBaseNameW(handle, module[0], &mut module_name) } > 0 {
                    module_name.retain(|&x| x != 0);
                    let name = OsString::from_wide(&module_name)
                        .to_string_lossy()
                        .into_owned();
                    let info = ProcessInfo { hwnd, pid, name };
                    process_map.insert(pid, info);
                }
            }
        }
    }

    true.into()
}

pub fn get_all_processes_info() -> Vec<ProcessInfo> {
    let mut process_map: HashMap<u32, ProcessInfo> = HashMap::new();
    unsafe {
        EnumWindows(
            Some(enum_window_callback),
            LPARAM(&mut process_map as *mut _ as isize),
        );
    }
    process_map.into_values().collect()
}

pub fn find_process_by_keyword(keyword: &str) -> Option<HWND> {
    let processes = get_all_processes_info();
    for process in processes {
        if process.name.contains(keyword) {
            return Some(process.hwnd);
        }
    }
    None
}

pub fn get_process_info(hwnd: HWND) -> ProcessInfo {
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }

    let process_handle =
        unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid) };
    if let Ok(handle) = process_handle {
        let mut module: [HMODULE; 1] = [HMODULE::default(); 1];
        let mut cb_needed = 0;
        if unsafe {
            K32EnumProcessModules(
                handle,
                module.as_mut_ptr(),
                std::mem::size_of_val(&module) as u32,
                &mut cb_needed,
            )
        }
        .as_bool()
        {
            let mut module_name = vec![0u16; 1024];
            if unsafe { K32GetModuleBaseNameW(handle, module[0], &mut module_name) } > 0 {
                module_name.retain(|&x| x != 0);
                let name = OsString::from_wide(&module_name)
                    .to_string_lossy()
                    .into_owned();
                return ProcessInfo { hwnd, pid, name };
            }
        }
    }
    ProcessInfo {
        hwnd,
        pid,
        name: String::new(),
    }
}

pub fn find_child_window_by_class(parent: HWND, class_name: &str) -> Option<HWND> {
    let mut result: Option<HWND> = None;
    let class_name_wide: Vec<u16> = OsString::from(class_name).encode_wide().collect();

    unsafe extern "system" fn enum_child_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let class_name_ptr = lparam.0 as *const Vec<u16>;
        let class_name = &*class_name_ptr;
        let mut buffer = vec![0u16; 256];
        let len = GetClassNameW(hwnd, &mut buffer) as usize;
        buffer.truncate(len);
        if buffer == *class_name {
            let result = &mut *(lparam.0 as *mut Option<HWND>);
            *result = Some(hwnd);
            return false.into(); // Stop enumerating
        }
        true.into() // Continue enumerating
    }

    unsafe {
        EnumChildWindows(
            parent,
            Some(enum_child_proc),
            LPARAM(&mut result as *mut _ as isize),
        );
    }
    result
}

pub fn get_window_text(hwnd: HWND) -> String {
    let mut buffer = vec![0u16; 1024];
    unsafe {
        let len = GetWindowTextW(hwnd, &mut buffer) as usize;
        buffer.truncate(len);
    }
    OsString::from_wide(&buffer).to_string_lossy().into_owned()
}
