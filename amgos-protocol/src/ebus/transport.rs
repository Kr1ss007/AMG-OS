//! AMGOS EventBus (e-bus) Unix Domain Socket Transport
//!
//! Provides production client/server communication over Unix domain sockets
//! with thread-safe connection management and framed message handling.

use std::fs;
use std::io;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::frame::{read_frame_header, read_frame_payload, write_frame, FrameError};
use super::messages::{DesktopRequest, SystemEvent};

pub const DEFAULT_EBUS_SOCKET_PATH: &str = "/tmp/amgos-ebus.sock";

/// Client used by Process 2 (Desktop Session) to communicate over e-bus
pub struct EventBusClient {
    stream: Mutex<UnixStream>,
    seq_counter: AtomicU32,
}

impl EventBusClient {
    pub fn connect<P: AsRef<Path>>(socket_path: P) -> Result<Self, FrameError> {
        let stream = UnixStream::connect(socket_path)?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;

        Ok(Self {
            stream: Mutex::new(stream),
            seq_counter: AtomicU32::new(1),
        })
    }

    /// Send a request from Process 2 to Process 1
    pub fn send_request(&self, request: &DesktopRequest) -> Result<u32, FrameError> {
        let msg_id = self.seq_counter.fetch_add(1, Ordering::SeqCst);
        let mut stream = self
            .stream
            .lock()
            .map_err(|_| FrameError::Io("Lock poisoned".into()))?;
        write_frame(&mut *stream, msg_id, request)?;
        Ok(msg_id)
    }

    /// Read the next SystemEvent emitted by Process 1
    pub fn read_event(&self) -> Result<SystemEvent, FrameError> {
        let mut stream = self
            .stream
            .lock()
            .map_err(|_| FrameError::Io("Lock poisoned".into()))?;
        let header = read_frame_header(&mut *stream)?;
        read_frame_payload(&mut *stream, &header)
    }
}

/// Server owned by Process 1 (System Session) to broker all e-bus messages
pub struct EventBusServer {
    socket_path: PathBuf,
    listener: UnixListener,
    subscribers: Arc<Mutex<Vec<UnixStream>>>,
    running: Arc<AtomicBool>,
    seq_counter: AtomicU32,
}

impl EventBusServer {
    pub fn bind<P: AsRef<Path>>(socket_path: P) -> Result<Self, io::Error> {
        let path = socket_path.as_ref().to_path_buf();
        if path.exists() {
            let _ = fs::remove_file(&path);
        }

        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let listener = UnixListener::bind(&path)?;
        listener.set_nonblocking(true)?;

        Ok(Self {
            socket_path: path,
            listener,
            subscribers: Arc::new(Mutex::new(Vec::new())),
            running: Arc::new(AtomicBool::new(true)),
            seq_counter: AtomicU32::new(1),
        })
    }

    /// Accept any new incoming connections (non-blocking)
    pub fn poll_connections(&self) -> Result<usize, io::Error> {
        let mut new_count = 0;
        loop {
            match self.listener.accept() {
                Ok((stream, _)) => {
                    let _ = stream.set_nonblocking(false);
                    let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));
                    if let Ok(mut subs) = self.subscribers.lock() {
                        subs.push(stream);
                        new_count += 1;
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }
        Ok(new_count)
    }

    /// Publish a SystemEvent to all connected subscribers in Process 2
    pub fn publish_event(&self, event: &SystemEvent) -> Result<usize, FrameError> {
        let msg_id = self.seq_counter.fetch_add(1, Ordering::SeqCst);
        let mut subs = self
            .subscribers
            .lock()
            .map_err(|_| FrameError::Io("Lock poisoned".into()))?;
        let mut delivered = 0;
        let mut retained = Vec::with_capacity(subs.len());

        for mut stream in subs.drain(..) {
            match write_frame(&mut stream, msg_id, event) {
                Ok(_) => {
                    delivered += 1;
                    retained.push(stream);
                }
                Err(_) => {
                    // Connection closed or broken pipe, discard stream
                }
            }
        }

        *subs = retained;
        Ok(delivered)
    }

    pub fn subscriber_count(&self) -> usize {
        self.subscribers.lock().map(|s| s.len()).unwrap_or(0)
    }

    pub fn shutdown(&self) {
        self.running.store(false, Ordering::SeqCst);
        let _ = fs::remove_file(&self.socket_path);
    }
}

impl Drop for EventBusServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ebus_pub_sub_roundtrip() {
        let socket_path = "/tmp/amgos_ebus_test_roundtrip.sock";
        let server = EventBusServer::bind(socket_path).expect("Server bind failed");

        let client = EventBusClient::connect(socket_path).expect("Client connect failed");

        // Let server poll and register the client
        let connected = server.poll_connections().expect("Poll failed");
        assert_eq!(connected, 1);
        assert_eq!(server.subscriber_count(), 1);

        let event = SystemEvent::BootChimeTrigger {
            timestamp_ns: 123456789,
            note: "F3".to_string(),
            frequency_hz: 174.61,
            duration_ms: 1500,
        };

        let delivered = server.publish_event(&event).expect("Publish failed");
        assert_eq!(delivered, 1);

        let received = client.read_event().expect("Read event failed");
        assert_eq!(event, received);
    }
}
