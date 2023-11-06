#![feature(naked_functions)]
#![allow(named_asm_labels)]
#![feature(asm_const)]

mod export_indices;
mod intercepted_exports;
mod orig_exports;
mod proxied_exports;

pub use intercepted_exports::*;
pub use proxied_exports::*;

use export_indices::TOTAL_EXPORTS;
use orig_exports::load_dll_funcs;
use std::arch::x86_64::_mm_pause;
use std::ffi::OsString;
use std::os::windows::prelude::{AsRawHandle, OsStringExt};
use winapi::ctypes::c_void;
use winapi::shared::minwindef::{DWORD, FARPROC, HMODULE};
use winapi::shared::ntdef::LPCSTR;
use winapi::um::consoleapi::AllocConsole;
use winapi::um::handleapi::CloseHandle;
use winapi::um::libloaderapi::{
    DisableThreadLibraryCalls, FreeLibrary, GetModuleFileNameW, LoadLibraryA,
};
use winapi::um::processenv::SetStdHandle;
use winapi::um::processthreadsapi::{
    CreateThread, GetCurrentProcess, OpenThread, ResumeThread, SuspendThread, TerminateProcess,
};
use winapi::um::tlhelp32::{
    CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
};
use winapi::um::winbase::{STD_ERROR_HANDLE, STD_OUTPUT_HANDLE};
use winapi::um::winnt::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH, THREAD_ALL_ACCESS};
use winapi::um::winuser::{MessageBoxA, MB_OK};

// Static handles
static mut THIS_HANDLE: Option<HMODULE> = None;
static mut ORIG_DLL_HANDLE: Option<HMODULE> = None;

// Original funcs
#[no_mangle]
pub static mut ORIGINAL_FUNCS: [FARPROC; TOTAL_EXPORTS] = [std::ptr::null_mut(); TOTAL_EXPORTS];
#[no_mangle]
pub static mut ORIG_FUNCS_PTR: *const FARPROC = std::ptr::null_mut();

/// Indicates once we are ready to accept incoming calls to proxied functions
static mut {{ package_name }}_READY: bool = false;

#[no_mangle]
pub unsafe extern "stdcall" fn DllMain(module: HMODULE, reason: u32, _res: *const c_void) -> i32 {
    DisableThreadLibraryCalls(module);
    THIS_HANDLE = Some(module);

    if reason == DLL_PROCESS_ATTACH {
        // suspend_all_threads(GetCurrentProcessId(), GetCurrentThreadId());
        CreateThread(
            std::ptr::null_mut(),
            0,
            Some(init),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
        );
    } else if reason == DLL_PROCESS_DETACH {
        if let Some(orig_dll_handle) = ORIG_DLL_HANDLE {
            println!("Freeing original DLL");
            FreeLibrary(orig_dll_handle);
        }
    }

    1
}

/// Get the current DLLs path
unsafe fn get_dll_path() -> Option<String> {
    let mut buffer: Vec<u16> = vec![0; 260];
    if THIS_HANDLE.is_none() {
        return None;
    }
    let size = GetModuleFileNameW(
        THIS_HANDLE.unwrap(),
        buffer.as_mut_ptr(),
        buffer.len() as u32,
    );

    if size == 0 {
        return None;
    }

    buffer.truncate(size as usize);
    let os_string = OsString::from_wide(&buffer);
    Some(os_string.to_string_lossy().into_owned())
}

unsafe fn die() {
    show_message("{{ package_name }}", "About to exit...");
    println!("Exiting...");
    TerminateProcess(GetCurrentProcess(), 0);
}

unsafe fn show_message(title: &str, message: &str) {
    let title = format!("{}\0", title);
    let message = format!("{}\0", message);
    MessageBoxA(
        std::ptr::null_mut(),
        message.as_bytes().as_ptr() as LPCSTR,
        title.as_bytes().as_ptr() as LPCSTR,
        MB_OK,
    );
}

/// Called when the thread is spawned
unsafe extern "system" fn init(_: *mut c_void) -> u32 {
    ORIG_FUNCS_PTR = ORIGINAL_FUNCS.as_ptr();
    AllocConsole();
    let stdout = std::io::stdout();
    let out_handle = stdout.as_raw_handle();
    let out_handle = out_handle as *mut c_void;
    SetStdHandle(STD_OUTPUT_HANDLE, out_handle);
    let stderr = std::io::stderr();
    let err_handle = stderr.as_raw_handle();
    let err_handle = err_handle as *mut c_void;
    SetStdHandle(STD_ERROR_HANDLE, err_handle);
    init_dlc_data();
    if let Some(dll_path) = get_dll_path() {
        println!("This DLL path: {}", &dll_path);
        let orig_dll_name = format!("{}_", &dll_path);
        ORIG_DLL_HANDLE = Some(LoadLibraryA(orig_dll_name.as_ptr() as *const i8));
    } else {
        show_message("{{ package_name }}", "Failed to get DLL path");
        eprint!("Failed to get DLL path");
        return 1;
    }
    if let Some(orig_dll_handle) = ORIG_DLL_HANDLE {
        if orig_dll_handle.is_null() {
            show_message("{{ package_name }}", "Failed to load original DLL");
            eprintln!("Failed to load original DLL");
            die();
        }
        println!("Original DLL handle: {:?}", orig_dll_handle);
    } else {
        show_message("{{ package_name }}", "Failed to load original DLL");
        eprintln!("Failed to load original DLL");
        die();
    }
    load_dll_funcs();
    {{ package_name }}_READY = true;
    0
}

#[no_mangle]
pub unsafe extern "C" fn wait_init() {
    // Spin a little while we wait for all functions to fully load
    // NOTE TO SELF: DO NO PRINT STUFF IN HERE
    while !{{ package_name }}_READY {
        _mm_pause();
    }
}

/// This should load up the DLC IDs to spoof
fn init_dlc_data() {
    println!("In init_dlc_data");
}
