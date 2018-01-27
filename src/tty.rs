// Copyright 2016 Joe Wilm, The Alacritty Project Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
//! tty related functionality
//!
use std::fs::File;
use std::os::unix::io::FromRawFd;
use std::os::unix::process::CommandExt;
use std::ptr;
use std::process::{Command, Stdio, Child};
use libc::{self, winsize, c_int, TIOCSCTTY};

/// Get the current value of errno
fn errno() -> c_int {
    ::errno::errno().0
}

/// Get raw fds for master/slave ends of a new pty
#[cfg(target_os = "linux")]
fn openpty(rows: u8, cols: u8) -> (c_int, c_int) {
    let mut master: c_int = 0;
    let mut slave: c_int = 0;

    let win = winsize {
        ws_row: libc::c_ushort::from(rows),
        ws_col: libc::c_ushort::from(cols),
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    let res = unsafe {
        libc::openpty(&mut master, &mut slave, ptr::null_mut(), ptr::null(), &win)
    };

    if res < 0 {
        die!("openpty failed");
    }

    (master, slave)
}

#[cfg(any(target_os = "macos",target_os = "freebsd"))]
fn openpty(rows: u8, cols: u8) -> (c_int, c_int) {
    let mut master: c_int = 0;
    let mut slave: c_int = 0;

    let mut win = winsize {
        ws_row: libc::c_ushort::from(rows),
        ws_col: libc::c_ushort::from(cols),
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    let res = unsafe {
        libc::openpty(&mut master, &mut slave, ptr::null_mut(), ptr::null_mut(), &mut win)
    };

    if res < 0 {
        die!("openpty failed");
    }

    (master, slave)
}

/// Really only needed on BSD, but should be fine elsewhere
fn set_controlling_terminal(fd: c_int) {
    let res = unsafe {
        // TIOSCTTY changes based on platform and the `ioctl` call is different
        // based on architecture (32/64). So a generic cast is used to make sure
        // there are no issues. To allow such a generic cast the clippy warning
        // is disabled.
        #[cfg_attr(feature = "clippy", allow(cast_lossless))]
        libc::ioctl(fd, TIOCSCTTY as _, 0)
    };

    if res < 0 {
        die!("ioctl TIOCSCTTY failed: {}", errno());
    }
}

use std::num::ParseIntError;

/// Create a new tty and return a handle to interact with it.
pub fn new(command: &Vec<String>, (rows, cols): (u16, u16)) -> Result<Pty, ParseIntError> {
    let win = winsize {
        ws_row: libc::c_ushort::from(rows),
        ws_col: libc::c_ushort::from(cols),
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    unsafe {libc::signal(libc::SIGPIPE, libc::SIG_IGN); };

    let (master, slave) = openpty(win.ws_row as _, win.ws_col as _);

    let mut builder = Command::new(&command[0]);
    for arg in command[1..].iter() {
        builder.arg(arg);
    }

    // Setup child stdin/stdout/stderr as slave fd of pty
    // Ownership of fd is transferred to the Stdio structs and will be closed by them at the end of
    // this scope. (It is not an issue that the fd is closed three times since File::drop ignores
    // error on libc::close.)
    builder.stdin(unsafe { Stdio::from_raw_fd(slave) });
    builder.stderr(unsafe { Stdio::from_raw_fd(slave) });
    builder.stdout(unsafe { Stdio::from_raw_fd(slave) });

    builder.before_exec(move || {
        // Create a new process group
        unsafe {
            let err = libc::setsid();
            if err == -1 {
                die!("Failed to set session id: {}", errno());
            }
        }

        set_controlling_terminal(slave);

        // No longer need slave/master fds
        unsafe {
            libc::close(slave);
            libc::close(master);
        }

        unsafe {
            libc::signal(libc::SIGCHLD, libc::SIG_DFL);
            libc::signal(libc::SIGHUP, libc::SIG_DFL);
            libc::signal(libc::SIGINT, libc::SIG_DFL);
            libc::signal(libc::SIGQUIT, libc::SIG_DFL);
            libc::signal(libc::SIGTERM, libc::SIG_DFL);
            libc::signal(libc::SIGALRM, libc::SIG_DFL);
        }
        Ok(())
    });

    match builder.spawn() {
        Ok(child) => {
            Ok (Pty { fd: master, child })
        },
        Err(err) => {
            die!("Command::spawn() failed: {}", err);
        }
    }
}

pub struct Pty {
    fd: c_int,
    pub child: Child,
}

impl Pty {
    pub fn key(&self) -> u32 {
        return self.fd as u32;
    }

    /// Get reader for the TTY
    ///
    /// XXX File is a bad abstraction here; it closes the fd on drop
    pub fn reader(&self) -> File {
        unsafe {
            File::from_raw_fd(self.fd)
        }
    }

    /// Resize the pty
    ///
    /// Tells the kernel that the window size changed with the new pixel
    /// dimensions and line/column counts.
    pub fn _resize(&self, win: &libc::winsize) {
        let res = unsafe {
            libc::ioctl(self.fd, libc::TIOCSWINSZ, &win as *const _)
        };

        if res < 0 {
            die!("ioctl TIOCSWINSZ failed: {}", errno());
        }
    }
}
