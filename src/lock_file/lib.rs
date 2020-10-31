#[cfg(test)]
mod tests {
    use crate::{lock_file::LockFile, user_config::UserConfig, LOCK_FILE};
    use std::{
        convert::TryInto,
        fs,
        io::{Error, ErrorKind, Result},
    };
    use sysinfo::{ProcessExt, RefreshKind, Signal, System, SystemExt};

    fn stop() {
        let sys = System::new_with_specifics(RefreshKind::with_processes(RefreshKind::new()));
        let lock_file = LockFile::new();
        let watchers = lock_file.get_running_watchers();
        for (pid, _) in watchers.iter() {
            sys.get_process(*pid).unwrap().kill(Signal::Kill);
        }
    }

    fn simulate_watch() {
        let pid = 1000000000i32;
        let sys = System::new_with_specifics(RefreshKind::with_processes(RefreshKind::new()));
        assert!(sys.get_process(pid).is_none());
        let path = UserConfig::default_path();
        LOCK_FILE.append(pid.try_into().unwrap(), &path).unwrap();
    }

    #[test]
    fn clear_dead_processes() -> Result<()> {
        stop();
        simulate_watch();
        let lock_file = LockFile::new();
        let content = fs::read_to_string(&lock_file.path).expect("couldnt read lockfile");
        if content.is_empty() {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::Other,
                "processes are not being cleared from lockfile properly",
            ))
        }
    }
}
