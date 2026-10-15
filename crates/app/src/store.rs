//! Where saves and settings are kept: files in the platform's data folder, or the browser's
//! `localStorage` when the game runs in a web page.

/// Report a problem: on standard error, or the browser's console.
pub fn warn(msg: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("{msg}");
    #[cfg(target_arch = "wasm32")]
    web_sys::console::error_1(&msg.into());
}

#[cfg(target_arch = "wasm32")]
pub use browser::Store;
#[cfg(not(target_arch = "wasm32"))]
pub use files::Store;

#[cfg(not(target_arch = "wasm32"))]
mod files {
    use std::path::PathBuf;

    #[derive(Clone)]
    pub struct Store {
        dir: PathBuf,
    }

    impl Store {
        /// The platform's data folder: `~/Library/Application Support/Mystery House` on macOS.
        pub fn default_dir() -> PathBuf {
            dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("Mystery House")
        }

        pub fn new(dir: PathBuf) -> Store {
            Store { dir }
        }

        pub fn read(&self, name: &str) -> Option<Vec<u8>> {
            std::fs::read(self.dir.join(name)).ok()
        }

        pub fn write(&self, name: &str, bytes: &[u8]) -> Result<(), String> {
            let path = self.dir.join(name);
            std::fs::create_dir_all(&self.dir)
                .and_then(|_| std::fs::write(&path, bytes))
                .map_err(|e| format!("{}: {e}", path.display()))
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod browser {
    /// `localStorage`, each file under `mystery-house/<name>` as hex.
    #[derive(Clone)]
    pub struct Store;

    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok()?
    }

    impl Store {
        pub fn read(&self, name: &str) -> Option<Vec<u8>> {
            let hex = storage()?
                .get_item(&format!("mystery-house/{name}"))
                .ok()??;
            (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok())
                .collect()
        }

        pub fn write(&self, name: &str, bytes: &[u8]) -> Result<(), String> {
            let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
            storage()
                .ok_or("this browser keeps no local storage")?
                .set_item(&format!("mystery-house/{name}"), &hex)
                .map_err(|_| format!("cannot keep {name} in local storage"))
        }
    }
}
