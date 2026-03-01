use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

pub struct SingleInstance {
    _file: File,
    lock_path: PathBuf,
}

impl SingleInstance {
    pub fn acquire(lock_path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(lock_path)?;
        Ok(Self {
            _file: file,
            lock_path: lock_path.to_path_buf(),
        })
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.lock_path);
    }
}
