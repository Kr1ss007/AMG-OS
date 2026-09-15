//! PTY Allocation and Terminal Controller
//!
//! Allocates POSIX pseudo-terminal (PTY) master/slave pairs,
//! spawns an interactive shell session attached to the slave terminal,
//! and provides non-blocking I/O streams for Terminow.

use std::fs::File;
use std::io::{self, Read, Write};
use std::os::unix::io::{FromRawFd, RawFd};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::ptr;

pub struct PtySession {
    master_fd: RawFd,
    slave_fd: RawFd,
    master_file: File,
    child: Option<Child>,
}

impl PtySession {
    pub fn open(cols: u16, rows: u16) -> Result<Self, io::Error> {
        let mut master: RawFd = -1;
        let mut slave: RawFd = -1;

        let win = libc::winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        let res = unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                ptr::null_mut(),
                ptr::null_mut(),
                &win as *const libc::winsize as *mut libc::winsize,
            )
        };

        if res != 0 {
            return Err(io::Error::last_os_error());
        }

        // Set master fd to non-blocking so reads don't block the Wayland event loop
        unsafe {
            let flags = libc::fcntl(master, libc::F_GETFL);
            if flags >= 0 {
                libc::fcntl(master, libc::F_SETFL, flags | libc::O_NONBLOCK);
            }
        }

        // Duplicate slave fd for stdout and stderr before passing ownership
        let slave_out = unsafe { libc::dup(slave) };
        let slave_err = unsafe { libc::dup(slave) };

        let shell = std::env::var("SHELL").unwrap_or_else(|_| {
            if Path::new("/bin/bash").exists() {
                "/bin/bash".to_string()
            } else {
                "/bin/sh".to_string()
            }
        });

        let child = unsafe {
            Command::new(&shell)
                .env("TERM", "xterm-256color")
                .env("COLORTERM", "truecolor")
                .stdin(Stdio::from_raw_fd(slave))
                .stdout(Stdio::from_raw_fd(slave_out))
                .stderr(Stdio::from_raw_fd(slave_err))
                .pre_exec(move || {
                    libc::setsid();
                    libc::ioctl(0, libc::TIOCSCTTY, 1);
                    Ok(())
                })
                .spawn()?
        };

        let master_file = unsafe { File::from_raw_fd(master) };

        Ok(Self {
            master_fd: master,
            slave_fd: slave,
            master_file,
            child: Some(child),
        })
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<(), io::Error> {
        let win = libc::winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        let res = unsafe { libc::ioctl(self.master_fd, libc::TIOCSWINSZ, &win) };
        if res != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    pub fn write_input(&mut self, data: &[u8]) -> io::Result<usize> {
        self.master_file.write(data)
    }

    pub fn read_output(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        match self.master_file.read(buffer) {
            Ok(n) => Ok(n),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(0),
            Err(e) => Err(e),
        }
    }

    pub fn slave_fd(&self) -> RawFd {
        self.slave_fd
    }

    pub fn is_alive(&mut self) -> bool {
        if let Some(ref mut child) = self.child {
            match child.try_wait() {
                Ok(None) => true,
                _ => false,
            }
        } else {
            false
        }
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
