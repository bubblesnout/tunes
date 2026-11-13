//! Steam overlay diagnostics (Windows). Logs, for Tunes and each WebView2 process it
//! spawned, whether Steam's overlay module and Direct3D are loaded. The overlay can only
//! draw in a process that has both, so this shows where (or whether) Steam got in.

#[cfg(windows)]
mod imp {
    use std::mem::size_of;
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Module32FirstW, Module32NextW, Process32FirstW, Process32NextW, MODULEENTRY32W,
        PROCESSENTRY32W, TH32CS_SNAPMODULE, TH32CS_SNAPMODULE32, TH32CS_SNAPPROCESS,
    };

    const INTERESTING: &[&str] = &["gameoverlayrenderer64.dll", "d3d11.dll", "d3d12.dll", "dxgi.dll", "opengl32.dll"];

    fn wide(s: &[u16]) -> String {
        let end = s.iter().position(|&c| c == 0).unwrap_or(s.len());
        String::from_utf16_lossy(&s[..end])
    }

    /// (pid, parent pid, exe name) for every process.
    fn processes() -> Vec<(u32, u32, String)> {
        let mut out = Vec::new();
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snap == INVALID_HANDLE_VALUE {
                return out;
            }
            let mut e: PROCESSENTRY32W = std::mem::zeroed();
            e.dwSize = size_of::<PROCESSENTRY32W>() as u32;
            let mut ok = Process32FirstW(snap, &mut e) != 0;
            while ok {
                out.push((e.th32ProcessID, e.th32ParentProcessID, wide(&e.szExeFile)));
                ok = Process32NextW(snap, &mut e) != 0;
            }
            CloseHandle(snap);
        }
        out
    }

    /// Which of the interesting modules a process has loaded (None if it can't be inspected).
    fn modules(pid: u32) -> Option<Vec<String>> {
        let mut found = Vec::new();
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid);
            if snap == INVALID_HANDLE_VALUE {
                return None;
            }
            let mut m: MODULEENTRY32W = std::mem::zeroed();
            m.dwSize = size_of::<MODULEENTRY32W>() as u32;
            let mut ok = Module32FirstW(snap, &mut m) != 0;
            while ok {
                let name = wide(&m.szModule).to_lowercase();
                if INTERESTING.contains(&name.as_str()) && !found.contains(&name) {
                    found.push(name);
                }
                ok = Module32NextW(snap, &mut m) != 0;
            }
            CloseHandle(snap);
        }
        found.sort();
        Some(found)
    }

    pub fn log(reason: &str) {
        let me = std::process::id();
        let procs = processes();
        // Tunes plus all descendants (WebView2 browser → renderer/utility/gpu processes).
        let mut tree = vec![(me, 0usize)];
        let mut i = 0;
        while i < tree.len() {
            let (pid, depth) = tree[i];
            for (child, parent, _) in &procs {
                if *parent == pid && !tree.iter().any(|(p, _)| p == child) {
                    tree.push((*child, depth + 1));
                }
            }
            i += 1;
        }
        log::info!("overlay diagnostics ({reason}):");
        for (pid, depth) in tree {
            let name = procs.iter().find(|(p, _, _)| *p == pid).map(|(_, _, n)| n.as_str()).unwrap_or("tunes");
            let mods = match modules(pid) {
                Some(m) if m.is_empty() => "none of interest".to_string(),
                Some(m) => m.join(", "),
                None => "can't inspect".to_string(),
            };
            log::info!("  {}{name} [{pid}]: {mods}", "  ".repeat(depth));
        }
    }
}

/// Log overlay diagnostics now (no-op off Windows).
pub fn log(reason: &str) {
    #[cfg(windows)]
    imp::log(reason);
    #[cfg(not(windows))]
    let _ = reason;
}
