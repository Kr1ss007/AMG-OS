//! PTY Allocation and Terminal Controller
//!
//! Allocates POSIX pseudo-terminal (PTY) master/slave pairs,
//! handles terminal size ioctls, and provides asynchronous byte streams.

use std::fs::File;
use std::io::{self, Read, Write};
use std::os::unix::io::{FromRawFd, RawFd};
use std::ptr;

pub struct PtySession {
    master_fd: RawFd,
    slave_fd: RawFd,
    master_file: File,
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

        let master_file = unsafe { File::from_raw_fd(master) };

        Ok(Self {
            master_fd: master,
            slave_fd: slave,
            master_file,
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
        self.master_file.read(buffer)
    }

    pub fn slave_fd(&self) -> RawFd {
        self.slave_fd
    }
}
