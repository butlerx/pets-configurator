use std::{env, fs, io, io::Write, process};

#[derive(Debug)]
pub struct Lock {
    path: String,
}

impl Lock {
    pub fn acquire() -> Result<Self, String> {
        let path = env::var("PETS_LOCK_FILE").unwrap_or_else(|_| "/tmp/pets.lock".to_string());
        Self::acquire_at(path)
    }

    fn acquire_at(path: String) -> Result<Self, String> {
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut f) => {
                let _ = writeln!(f, "{}", process::id());
                Ok(Self { path })
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Err(format!(
                "another pets instance is running (lock file: {path})"
            )),
            Err(e) => {
                log::warn!("could not create lock file {path}: {e}");
                Ok(Self { path })
            }
        }
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn temp_lock_path() -> String {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.keep().join("test-pets.lock");
        path.to_str().unwrap().to_string()
    }

    #[test]
    fn acquire_creates_lock_file() {
        let path = temp_lock_path();
        let lock = Lock::acquire_at(path.clone()).unwrap();
        assert!(Path::new(&path).exists());
        drop(lock);
    }

    #[test]
    fn drop_removes_lock_file() {
        let path = temp_lock_path();
        let lock = Lock::acquire_at(path.clone()).unwrap();
        assert!(Path::new(&path).exists());
        drop(lock);
        assert!(!Path::new(&path).exists());
    }

    #[test]
    fn second_acquire_fails_while_held() {
        let path = temp_lock_path();
        let _lock = Lock::acquire_at(path.clone()).unwrap();
        let result = Lock::acquire_at(path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("another pets instance"));
    }

    #[test]
    fn acquire_succeeds_after_release() {
        let path = temp_lock_path();
        let lock = Lock::acquire_at(path.clone()).unwrap();
        drop(lock);
        assert!(Lock::acquire_at(path).is_ok());
    }

    #[test]
    fn lock_file_contains_pid() {
        let path = temp_lock_path();
        let _lock = Lock::acquire_at(path.clone()).unwrap();
        let contents = fs::read_to_string(&path).unwrap();
        let pid: u32 = contents.trim().parse().unwrap();
        assert_eq!(pid, process::id());
    }
}
