#[cfg(target_os = "windows")]
mod windows_impl {
    use std::path::Path;
    use std::process::Command;

    pub fn register_file_association(exe_path: &Path) -> Result<(), String> {
        let extension = "minigamesreplay";
        let prog_id = "MiniGames.Replay";
        let description = "Mini Games Replay File";

        let exe_path_str = exe_path.to_string_lossy();
        let open_command = format!("\"{}\" \"%1\"", exe_path_str);
        let icon_value = format!("\"{}\",0", exe_path_str);

        let key_ext = format!(r"HKCU\Software\Classes\.{}", extension);
        let key_prog = format!(r"HKCU\Software\Classes\{}", prog_id);
        let key_command = format!(r"HKCU\Software\Classes\{}\shell\open\command", prog_id);
        let key_icon = format!(r"HKCU\Software\Classes\{}\DefaultIcon", prog_id);

        let reg_commands: Vec<(&str, &str)> = vec![
            (&key_ext, prog_id),
            (&key_prog, description),
            (&key_command, &open_command),
            (&key_icon, &icon_value),
        ];

        for (key, value) in reg_commands {
            let output = Command::new("reg")
                .args(["add", key, "/ve", "/d", value, "/f"])
                .output()
                .map_err(|e| format!("Failed to execute reg command: {}", e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                common::log!("Registry command warning for {}: {}", key, stderr);
            }
        }

        notify_shell_change();

        common::log!("File association registered for .{}", extension);
        Ok(())
    }

    fn notify_shell_change() {
        let _ = Command::new("ie4uinit.exe")
            .arg("-show")
            .output();
    }

}

#[cfg(not(target_os = "windows"))]
mod other_impl {
    use std::path::Path;

    pub fn register_file_association(_exe_path: &Path) -> Result<(), String> {
        common::log!("File association registration is not supported on this platform");
        Ok(())
    }
}

#[cfg(target_os = "windows")]
pub use windows_impl::*;

#[cfg(not(target_os = "windows"))]
pub use other_impl::*;
