use std::path::Path;

pub trait IsHidden {
    fn is_hidden(&self) -> bool;
}

impl IsHidden for Path {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn is_hidden(&self) -> bool {
        self.file_name().unwrap().to_str().unwrap().starts_with('.')
    }

    #[cfg(target_os = "windows")]
    fn is_hidden(&self) -> bool {
        // must use winapi
        unimplemented!()
    }
}
