use std::path::PathBuf;

pub fn app_identifier(identifier: &str) -> String {
    let Some(root) = theseus::portable::root() else {
        return identifier.to_owned();
    };
    use sha2::{Digest, Sha256};
    let mut path = root.to_string_lossy().into_owned();
    if cfg!(windows) {
        path = path.replace('\\', "/").to_lowercase();
    }
    let digest = format!("{:x}", Sha256::digest(path.as_bytes()));
    // Tauri's single-instance plugin keys its mutex by this identifier. Each
    // portable folder must open its own data even if an installed copy runs.
    format!("{identifier}.portable{}", &digest[..16])
}

/// Must run before any other threads, including the Tokio runtime, are started.
/// Environment variables are consumed by the database, window-state plugin and
/// WebView2 before the application UI is created.
pub unsafe fn init_portable_mode() -> std::io::Result<bool> {
    let exe_path = std::env::current_exe()?;
    let Some(app_dir) = exe_path.parent() else {
        return Ok(false);
    };
    let Some(data_dir) = theseus::portable::prepare_directory(app_dir)? else {
        return Ok(false);
    };

    let webview_data = data_dir.join("webview");
    std::fs::create_dir_all(&webview_data)?;
    let runtime = app_dir.join("WebView2");
    #[cfg(windows)]
    if runtime.join("msedgewebview2.exe").is_file() {
        prepare_webview_runtime(&runtime)?;
    }

    // SAFETY: The caller guarantees that no other threads have been started.
    unsafe {
        std::env::set_var("THESEUS_CONFIG_DIR", &data_dir);
        #[cfg(windows)]
        {
            std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", &webview_data);
            if runtime.join("msedgewebview2.exe").is_file() {
                std::env::set_var(
                    "WEBVIEW2_BROWSER_EXECUTABLE_FOLDER",
                    &runtime,
                );
            }
        }
    }
    #[cfg(not(windows))]
    let _ = runtime;
    theseus::portable::initialize(PathBuf::from(app_dir));
    Ok(true)
}

#[cfg(windows)]
fn prepare_webview_runtime(runtime: &std::path::Path) -> std::io::Result<()> {
    use std::os::windows::{ffi::OsStrExt, process::CommandExt};
    use windows::Win32::Storage::FileSystem::{
        GetVolumeInformationW, GetVolumePathNameW,
    };
    use windows::Win32::System::SystemServices::FILE_PERSISTENT_ACLS;
    use windows::core::PCWSTR;

    let path: Vec<u16> = runtime.as_os_str().encode_wide().chain([0]).collect();
    let mut volume = vec![0u16; 32768];
    let mut flags = 0;
    // SAFETY: The path is NUL-terminated and output buffers remain valid for
    // each call. No pointers escape these synchronous Win32 functions.
    unsafe {
        GetVolumePathNameW(PCWSTR(path.as_ptr()), &mut volume)
            .map_err(std::io::Error::other)?;
        GetVolumeInformationW(
            PCWSTR(volume.as_ptr()),
            None,
            None,
            None,
            Some(&mut flags),
            None,
        )
        .map_err(std::io::Error::other)?;
    }
    // FAT/exFAT drives have no ACLs to configure.
    if flags & FILE_PERSISTENT_ACLS == 0 {
        return Ok(());
    }

    // Fixed WebView2 v120+ requires these inherited AppContainer read/execute
    // permissions on Windows 10. ZIP extraction does not retain NTFS ACLs.
    // https://learn.microsoft.com/microsoft-edge/webview2/concepts/distribution
    let system = std::env::var_os("SystemRoot").ok_or_else(|| {
        std::io::Error::other("The Windows system directory is unavailable")
    })?;
    let output = std::process::Command::new(
        PathBuf::from(system).join("System32/icacls.exe"),
    )
    .arg(runtime)
    .args([
        "/grant",
        "*S-1-15-2-2:(OI)(CI)(RX)",
        "*S-1-15-2-1:(OI)(CI)(RX)",
    ])
    .creation_flags(0x08000000) // CREATE_NO_WINDOW
    .output()?;
    if !output.status.success() {
        // Another PC's user may lack WRITE_DAC while these universal SIDs
        // already have access. WebView2 checks the effective permissions.
        eprintln!(
            "Could not update WebView2 runtime permissions: {} ({})",
            runtime.display(),
            output.status,
        );
    }
    Ok(())
}

#[tauri::command]
pub fn is_portable_mode() -> bool {
    theseus::portable::is_enabled()
}
