#[cfg(target_os = "windows")]
mod windows_impl {
    use std::path::Path;

    pub fn register_file_association(exe_path: &Path) -> Result<(), String> {
        use std::process::Command;

        let extension = "minigamesreplay";
        let prog_id = "MiniGames.Replay";
        let description = "Mini Games Replay File";

        let exe_path_str = exe_path.to_string_lossy();

        let commands = [
            format!(
                r#"reg add "HKCU\Software\Classes\.{}" /ve /d "{}" /f"#,
                extension, prog_id
            ),
            format!(
                r#"reg add "HKCU\Software\Classes\{}" /ve /d "{}" /f"#,
                prog_id, description
            ),
            format!(
                r#"reg add "HKCU\Software\Classes\{}\shell\open\command" /ve /d "\"{}\" \"%1\"" /f"#,
                prog_id, exe_path_str
            ),
            format!(
                r#"reg add "HKCU\Software\Classes\{}\DefaultIcon" /ve /d "\"{}\",0" /f"#,
                prog_id, exe_path_str
            ),
        ];

        for cmd in &commands {
            let output = Command::new("cmd")
                .args(["/C", cmd])
                .output()
                .map_err(|e| format!("Failed to execute reg command: {}", e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                common::log!("Registry command warning: {}", stderr);
            }
        }

        notify_shell_change();

        common::log!("File association registered for .{}", extension);
        Ok(())
    }

    fn notify_shell_change() {
        use std::process::Command;

        let _ = Command::new("cmd")
            .args(["/C", "ie4uinit.exe", "-show"])
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
