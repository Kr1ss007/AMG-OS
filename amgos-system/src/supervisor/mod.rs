//! Process 1 Service Supervisor
//!
//! Supervises the lifecycle of all internal background subsystems in Process 1.
//! Guarantees clean startup, continuous health monitoring, and controlled shutdown.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub struct ServiceSupervisor {
    is_running: Arc<AtomicBool>,
    worker_threads: Vec<JoinHandle<()>>,
}

impl ServiceSupervisor {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(true)),
            worker_threads: Vec::new(),
        }
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    pub fn register_service<F>(&mut self, name: &'static str, mut task: F)
    where
        F: FnMut(&AtomicBool) + Send + 'static,
    {
        let running_flag = Arc::clone(&self.is_running);
        let handle = thread::Builder::new()
            .name(name.to_string())
            .spawn(move || {
                while running_flag.load(Ordering::SeqCst) {
                    task(&running_flag);
                    thread::sleep(Duration::from_millis(50));
                }
            })
            .expect("Failed to spawn supervisor worker thread");

        self.worker_threads.push(handle);
    }

    pub fn shutdown(&mut self) {
        self.is_running.store(false, Ordering::SeqCst);
        while let Some(handle) = self.worker_threads.pop() {
            let _ = handle.join();
        }
    }
}

impl Default for ServiceSupervisor {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ServiceSupervisor {
    fn drop(&mut self) {
        self.shutdown();
    }
}
