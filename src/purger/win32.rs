use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, LUID};
use windows_sys::Win32::Security::{
    AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES, TOKEN_ADJUST_PRIVILEGES,
    TOKEN_PRIVILEGES, TOKEN_QUERY, SE_PRIVILEGE_ENABLED,
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::ProcessStatus::K32EmptyWorkingSet;
use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, OpenProcessToken, SetProcessWorkingSetSize, PROCESS_ALL_ACCESS,
    PROCESS_QUERY_INFORMATION, PROCESS_SET_QUOTA,
};
use windows_sys::Win32::UI::Shell::IsUserAnAdmin;

#[link(name = "ntdll")]
extern "system" {
    fn NtSetSystemInformation(
        SystemInformationClass: u32,
        SystemInformation: *mut std::ffi::c_void,
        SystemInformationLength: u32,
    ) -> i32;
}

#[repr(C)]
struct SystemFileCacheInformation {
    current_size: usize,
    peak_size: usize,
    page_fault_count: u32,
    minimum_working_set: usize,
    maximum_working_set: usize,
    current_size_including_transition: usize,
    peak_size_including_transition: usize,
    transition_repurpose_count: u32,
    flags: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryStats {
    pub total_mb: u64,
    pub used_mb: u64,
    pub free_mb: u64,
    pub usage_percent: f32,
}

pub fn is_admin() -> bool {
    unsafe { IsUserAnAdmin() != 0 }
}

pub fn get_memory_stats() -> MemoryStats {
    unsafe {
        let mut status: MEMORYSTATUSEX = std::mem::zeroed();
        status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        if GlobalMemoryStatusEx(&mut status) != 0 {
            let total_mb = status.ullTotalPhys / (1024 * 1024);
            let free_mb = status.ullAvailPhys / (1024 * 1024);
            let used_mb = total_mb.saturating_sub(free_mb);
            let usage_percent = status.dwMemoryLoad as f32;
            MemoryStats {
                total_mb,
                used_mb,
                free_mb,
                usage_percent,
            }
        } else {
            MemoryStats {
                total_mb: 0,
                used_mb: 0,
                free_mb: 0,
                usage_percent: 0.0,
            }
        }
    }
}

pub fn enable_privilege(privilege_name: &str) -> bool {
    unsafe {
        let mut token: HANDLE = std::mem::zeroed();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut token) == 0 {
            return false;
        }

        let wide_name: Vec<u16> = OsStr::new(privilege_name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut luid: LUID = std::mem::zeroed();
        if LookupPrivilegeValueW(null(), wide_name.as_ptr(), &mut luid) == 0 {
            CloseHandle(token);
            return false;
        }

        let mut tp = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            }],
        };

        let res = AdjustTokenPrivileges(
            token,
            0,
            &mut tp,
            std::mem::size_of::<TOKEN_PRIVILEGES>() as u32,
            null_mut(),
            null_mut(),
        );

        CloseHandle(token);
        res != 0
    }
}

pub fn enable_all_privileges() {
    enable_privilege("SeDebugPrivilege");
    enable_privilege("SeProfileSingleProcessPrivilege");
    enable_privilege("SeIncreaseQuotaPrivilege");
    enable_privilege("SeSystemProfilePrivilege");
}

pub fn purge_working_sets() -> usize {
    enable_all_privileges();
    let mut processes_purged = 0;

    let mut cmd_empty_all: u32 = 2;
    unsafe {
        NtSetSystemInformation(
            80,
            &mut cmd_empty_all as *mut u32 as *mut std::ffi::c_void,
            std::mem::size_of::<u32>() as u32,
        );
    }

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot != null_mut() && snapshot != -1isize as HANDLE {
            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

            if Process32FirstW(snapshot, &mut entry) != 0 {
                loop {
                    let pid = entry.th32ProcessID;
                    if pid > 4 {
                        let mut h_proc = OpenProcess(PROCESS_SET_QUOTA | PROCESS_QUERY_INFORMATION, 0, pid);
                        if h_proc.is_null() {
                            h_proc = OpenProcess(PROCESS_SET_QUOTA, 0, pid);
                        }
                        if h_proc.is_null() {
                            h_proc = OpenProcess(PROCESS_ALL_ACCESS, 0, pid);
                        }

                        if !h_proc.is_null() {
                            let _ = K32EmptyWorkingSet(h_proc);
                            let _ = SetProcessWorkingSetSize(h_proc, usize::MAX, usize::MAX);
                            CloseHandle(h_proc);
                            processes_purged += 1;
                        }
                    }
                    if Process32NextW(snapshot, &mut entry) == 0 {
                        break;
                    }
                }
            }
            CloseHandle(snapshot);
        }
    }
    processes_purged
}

pub fn purge_standby_list() -> Result<(), String> {
    enable_all_privileges();
    let commands = [4u32, 8u32, 3u32];
    let mut last_status = 0i32;

    for &cmd in &commands {
        let mut command = cmd;
        let status = unsafe {
            NtSetSystemInformation(
                80,
                &mut command as *mut u32 as *mut std::ffi::c_void,
                std::mem::size_of::<u32>() as u32,
            )
        };
        if status >= 0 {
            return Ok(());
        }
        last_status = status;
    }
    Err(format!("Error NtSetSystemInformation (Standby): 0x{:X}", last_status as u32))
}

pub fn purge_modified_list() -> Result<(), String> {
    enable_all_privileges();
    let commands = [5u32, 4u32];
    let mut last_status = 0i32;

    for &cmd in &commands {
        let mut command = cmd;
        let status = unsafe {
            NtSetSystemInformation(
                80,
                &mut command as *mut u32 as *mut std::ffi::c_void,
                std::mem::size_of::<u32>() as u32,
            )
        };
        if status >= 0 {
            return Ok(());
        }
        last_status = status;
    }
    Err(format!("Error NtSetSystemInformation (Modified): 0x{:X}", last_status as u32))
}

pub fn purge_system_cache() -> Result<(), String> {
    enable_all_privileges();
    let mut info = SystemFileCacheInformation {
        current_size: 0,
        peak_size: 0,
        page_fault_count: 0,
        minimum_working_set: usize::MAX,
        maximum_working_set: usize::MAX,
        current_size_including_transition: 0,
        peak_size_including_transition: 0,
        transition_repurpose_count: 0,
        flags: 0,
    };
    let status = unsafe {
        NtSetSystemInformation(
            21,
            &mut info as *mut _ as *mut std::ffi::c_void,
            std::mem::size_of::<SystemFileCacheInformation>() as u32,
        )
    };
    if status >= 0 {
        Ok(())
    } else {
        Err(format!("Error NtSetSystemInformation (SystemCache): 0x{:X}", status))
    }
}
