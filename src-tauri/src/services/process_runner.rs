use portable_pty::{CommandBuilder, PtySize, native_pty_system, PtySystem, MasterPty, Child};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use uuid::Uuid;

/// Tracked process with PTY
struct TrackedProcess {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    reader_handle: Option<thread::JoinHandle<()>>,
}

/// Process runner managing PTY-based subprocesses
pub struct ProcessRunner {
    processes: Arc<Mutex<HashMap<String, TrackedProcess>>>,
}

impl ProcessRunner {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Spawn a command with PTY. Returns process ID.
    /// `on_output` is called with (process_id, text) for each chunk of output.
    pub fn spawn<F>(
        &self,
        program: &str,
        args: &[&str],
        cwd: Option<&str>,
        env: Option<HashMap<String, String>>,
        on_output: F,
    ) -> Result<String, String>
    where
        F: Fn(&str, &str) + Send + 'static,
    {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to open PTY: {}", e))?;

        let mut cmd = CommandBuilder::new(program);
        for arg in args {
            cmd.arg(*arg);
        }
        if let Some(dir) = cwd {
            cmd.cwd(dir);
        }
        if let Some(env_vars) = env {
            for (k, v) in env_vars {
                cmd.env(k, v);
            }
        }

        let child = pair.slave
            .spawn_command(cmd)
            .map_err(|e| format!("Failed to spawn: {}", e))?;

        let process_id = Uuid::new_v4().to_string();
        let pid = process_id.clone();

        // Read output in background thread
        let mut reader = pair.master
            .try_clone_reader()
            .map_err(|e| format!("Failed to clone reader: {}", e))?;

        let reader_handle = thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if let Ok(text) = String::from_utf8(buf[..n].to_vec()) {
                            on_output(&pid, &text);
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let tracked = TrackedProcess {
            master: pair.master,
            child,
            reader_handle: Some(reader_handle),
        };

        self.processes.lock().unwrap().insert(process_id.clone(), tracked);
        Ok(process_id)
    }

    /// Send input to a running process
    pub fn send_input(&self, process_id: &str, text: &str) -> Result<(), String> {
        let mut procs = self.processes.lock().unwrap();
        let proc = procs.get_mut(process_id).ok_or("Process not found")?;
        let mut writer = proc.master.take_writer().map_err(|e| format!("Writer error: {}", e))?;
        writer.write_all(text.as_bytes()).map_err(|e| format!("Write error: {}", e))?;
        writer.flush().map_err(|e| format!("Flush error: {}", e))?;
        Ok(())
    }

    /// Kill a process
    pub fn kill(&self, process_id: &str) -> Result<(), String> {
        let mut procs = self.processes.lock().unwrap();
        if let Some(mut proc) = procs.remove(process_id) {
            proc.child.kill().map_err(|e| format!("Kill error: {}", e))?;
            if let Some(handle) = proc.reader_handle.take() {
                let _ = handle.join();
            }
        }
        Ok(())
    }

    /// Wait for process to exit and return exit code
    pub fn wait(&self, process_id: &str) -> Result<u32, String> {
        let mut procs = self.processes.lock().unwrap();
        if let Some(mut proc) = procs.remove(process_id) {
            let status = proc.child.wait().map_err(|e| format!("Wait error: {}", e))?;
            if let Some(handle) = proc.reader_handle.take() {
                let _ = handle.join();
            }
            Ok(status.exit_code())
        } else {
            Err("Process not found".to_string())
        }
    }

    /// List active process IDs
    pub fn list_active(&self) -> Vec<String> {
        self.processes.lock().unwrap().keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn test_spawn_and_capture_output() {
        let runner = ProcessRunner::new();
        let got_output = Arc::new(AtomicBool::new(false));
        let flag = got_output.clone();

        let pid = runner.spawn(
            "echo",
            &["hello from pty"],
            None,
            None,
            move |_id, text| {
                if text.contains("hello from pty") {
                    flag.store(true, Ordering::SeqCst);
                }
            },
        ).unwrap();

        let exit_code = runner.wait(&pid).unwrap();
        assert_eq!(exit_code, 0);
        assert!(got_output.load(Ordering::SeqCst), "Should have captured output");
    }

    #[test]
    fn test_kill_process() {
        let runner = ProcessRunner::new();
        let pid = runner.spawn("sleep", &["60"], None, None, |_, _| {}).unwrap();
        assert_eq!(runner.list_active().len(), 1);

        runner.kill(&pid).unwrap();
        assert_eq!(runner.list_active().len(), 0);
    }
}
