use std::path::PathBuf;

pub fn get_app_data_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .expect("home dir")
            .join("Library/Application Support")
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .expect("home dir")
                    .join("AppData/Roaming")
            })
    }
    #[cfg(target_os = "linux")]
    {
        std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .expect("home dir")
                    .join(".local/share")
            })
    }
}

pub fn get_vscode_data_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .expect("home dir")
            .join("Library/Application Support/Code/User/globalStorage")
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA")
            .map(|p| PathBuf::from(p).join("Code/User/globalStorage"))
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .expect("home dir")
                    .join("AppData/Roaming/Code/User/globalStorage")
            })
    }
    #[cfg(target_os = "linux")]
    {
        std::env::var("XDG_CONFIG_HOME")
            .map(|p| PathBuf::from(p).join("Code/User/globalStorage"))
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .expect("home dir")
                    .join(".config/Code/User/globalStorage")
            })
    }
}
