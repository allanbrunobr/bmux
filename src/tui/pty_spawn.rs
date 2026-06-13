//! PTY spawn with selective fd preservation (secret / IPC key injection).
//!
//! `portable-pty` calls `close_random_fds()` in `pre_exec`, closing all fds ≥ 3.
//! This module reimplements spawn so caller-specified fds survive into the child.

#[cfg(unix)]
use std::io::{Read, Write};
#[cfg(unix)]
use std::mem;
#[cfg(unix)]
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
#[cfg(unix)]
use std::sync::Mutex;

use anyhow::{Context, Result};
use libc::{self, winsize};
use portable_pty::{
    Child, ChildKiller, CommandBuilder, ExitStatus, MasterPty, PtySize,
};

/// Result of spawning a command on a new PTY with optional preserved fds.
pub struct PtySpawnResult {
    pub master: Box<dyn MasterPty + Send>,
    pub child: Box<dyn Child + Send>,
    /// Parent-side copies of fds dup'd into the child — close after spawn returns.
    pub parent_fds_to_close: Vec<RawFd>,
}

/// Close every open fd numbered > 2 except those listed in `preserve`.
#[cfg(unix)]
pub fn close_fds_except(preserve: &[RawFd]) {
    if let Ok(dir) = std::fs::read_dir("/dev/fd") {
        let mut fds = Vec::new();
        for entry in dir.flatten() {
            if let Some(num) = entry
                .file_name()
                .into_string()
                .ok()
                .and_then(|n| n.parse::<RawFd>().ok())
            {
                if num > 2 && !preserve.contains(&num) {
                    fds.push(num);
                }
            }
        }
        for fd in fds {
            unsafe {
                libc::close(fd);
            }
        }
    }
}

#[cfg(unix)]
struct RawMaster {
    fd: OwnedFd,
    took_writer: Mutex<bool>,
}

#[cfg(unix)]
impl RawMaster {
    fn resize_fd(fd: RawFd, size: PtySize) -> Result<()> {
        let ws = winsize {
            ws_row: size.rows,
            ws_col: size.cols,
            ws_xpixel: size.pixel_width,
            ws_ypixel: size.pixel_height,
        };
        if unsafe { libc::ioctl(fd, libc::TIOCSWINSZ as _, &ws as *const _) } != 0 {
            return Err(std::io::Error::last_os_error()).context("ioctl(TIOCSWINSZ)");
        }
        Ok(())
    }
}

#[cfg(unix)]
impl MasterPty for RawMaster {
    fn resize(&self, size: PtySize) -> Result<(), anyhow::Error> {
        Self::resize_fd(self.fd.as_raw_fd(), size)?;
        Ok(())
    }

    fn get_size(&self) -> Result<PtySize, anyhow::Error> {
        let mut ws: winsize = unsafe { mem::zeroed() };
        if unsafe {
            libc::ioctl(
                self.fd.as_raw_fd(),
                libc::TIOCGWINSZ as _,
                &mut ws as *mut _,
            )
        } != 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(PtySize {
            rows: ws.ws_row,
            cols: ws.ws_col,
            pixel_width: ws.ws_xpixel,
            pixel_height: ws.ws_ypixel,
        })
    }

    fn try_clone_reader(&self) -> Result<Box<dyn Read + Send>, anyhow::Error> {
        let dup = unsafe {
            let raw = libc::dup(self.fd.as_raw_fd());
            if raw == -1 {
                return Err(std::io::Error::last_os_error().into());
            }
            OwnedFd::from_raw_fd(raw)
        };
        Ok(Box::new(std::fs::File::from(dup)))
    }

    fn take_writer(&self) -> Result<Box<dyn Write + Send>, anyhow::Error> {
        let mut took = self.took_writer.lock().expect("writer mutex");
        if *took {
            anyhow::bail!("cannot take writer more than once");
        }
        *took = true;
        let dup = unsafe {
            let raw = libc::dup(self.fd.as_raw_fd());
            if raw == -1 {
                return Err(std::io::Error::last_os_error().into());
            }
            OwnedFd::from_raw_fd(raw)
        };
        Ok(Box::new(std::fs::File::from(dup)))
    }

    fn process_group_leader(&self) -> Option<libc::pid_t> {
        match unsafe { libc::tcgetpgrp(self.fd.as_raw_fd()) } {
            pid if pid > 0 => Some(pid),
            _ => None,
        }
    }

    fn as_raw_fd(&self) -> Option<RawFd> {
        Some(self.fd.as_raw_fd())
    }
}

#[cfg(unix)]
#[derive(Debug)]
struct ProcessChild {
    inner: std::process::Child,
}

#[cfg(unix)]
impl Child for ProcessChild {
    fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>> {
        match self.inner.try_wait()? {
            Some(status) => Ok(Some(status.into())),
            None => Ok(None),
        }
    }

    fn wait(&mut self) -> std::io::Result<ExitStatus> {
        Ok(self.inner.wait()?.into())
    }

    fn process_id(&self) -> Option<u32> {
        Some(self.inner.id())
    }
}

#[cfg(unix)]
impl ChildKiller for ProcessChild {
    fn kill(&mut self) -> std::io::Result<()> {
        self.inner.kill()
    }

    fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
        Box::new(ProcessChildKiller(Some(self.inner.id())))
    }
}

#[cfg(unix)]
#[derive(Debug)]
struct ProcessChildKiller(Option<u32>);

#[cfg(unix)]
impl ChildKiller for ProcessChildKiller {
    fn kill(&mut self) -> std::io::Result<()> {
        if let Some(pid) = self.0 {
            let rc = unsafe { libc::kill(pid as i32, libc::SIGKILL) };
            if rc != 0 {
                return Err(std::io::Error::last_os_error());
            }
        }
        Ok(())
    }

    fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
        Box::new(ProcessChildKiller(self.0))
    }
}

#[cfg(unix)]
fn command_from_builder(builder: &CommandBuilder) -> Result<std::process::Command> {
    let argv = builder.get_argv();
    if argv.is_empty() {
        anyhow::bail!("spawn command has empty argv");
    }
    let mut command = std::process::Command::new(&argv[0]);
    for arg in argv.iter().skip(1) {
        command.arg(arg);
    }
    if let Some(cwd) = builder.get_cwd() {
        command.current_dir(cwd);
    }
    for (key, value) in builder.iter_full_env_as_str() {
        command.env(key, value);
    }
    Ok(command)
}

/// Open a PTY and spawn `cmd`, preserving `inject_fds` in the child at slots 3..N.
#[cfg(unix)]
pub fn spawn_with_preserved_fds(
    rows: u16,
    cols: u16,
    cmd: CommandBuilder,
    inject_fds: Vec<RawFd>,
) -> Result<PtySpawnResult> {
    let mut master_fd: RawFd = -1;
    let mut slave_fd: RawFd = -1;
    let mut ws = winsize {
        ws_row: rows,
        ws_col: cols,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    if unsafe {
        libc::openpty(
            &mut master_fd,
            &mut slave_fd,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut ws,
        )
    } != 0
    {
        return Err(std::io::Error::last_os_error()).context("openpty");
    }

    for fd in [master_fd, slave_fd] {
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
        if flags != -1 {
            unsafe {
                libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC);
            }
        }
    }

    let controlling_tty = cmd.get_controlling_tty();
    let mut command = command_from_builder(&cmd)?;
    let slave_for_stdio = slave_fd;
    let inject = inject_fds.clone();

    use std::process::Stdio;
    unsafe {
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .pre_exec(move || {
                for signo in &[
                    libc::SIGCHLD,
                    libc::SIGHUP,
                    libc::SIGINT,
                    libc::SIGQUIT,
                    libc::SIGTERM,
                    libc::SIGALRM,
                ] {
                    libc::signal(*signo, libc::SIG_DFL);
                }

                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }

                for std_fd in [0i32, 1, 2] {
                    if libc::dup2(slave_for_stdio, std_fd) == -1 {
                        return Err(std::io::Error::last_os_error());
                    }
                }

                if controlling_tty && libc::ioctl(0, libc::TIOCSCTTY as _, 0) == -1 {
                    return Err(std::io::Error::last_os_error());
                }

                let mut keep = vec![0, 1, 2];
                for (i, &src) in inject.iter().enumerate() {
                    let target = 3 + i as RawFd;
                    if libc::dup2(src, target) == -1 {
                        return Err(std::io::Error::last_os_error());
                    }
                    keep.push(target);
                    if src != target && libc::close(src) == -1 {
                        return Err(std::io::Error::last_os_error());
                    }
                }

                close_fds_except(&keep);

                Ok(())
            });
    }

    let child = command.spawn().context("spawn PTY child")?;
    unsafe {
        libc::close(slave_fd);
    }

    let master = RawMaster {
        fd: unsafe { OwnedFd::from_raw_fd(master_fd) },
        took_writer: Mutex::new(false),
    };

    Ok(PtySpawnResult {
        master: Box::new(master),
        child: Box::new(ProcessChild { inner: child }),
        parent_fds_to_close: inject_fds,
    })
}

/// Non-Unix fallback — delegates to portable-pty without fd preservation.
#[cfg(not(unix))]
pub fn spawn_with_preserved_fds(
    rows: u16,
    cols: u16,
    cmd: CommandBuilder,
    _inject_fds: Vec<i32>,
) -> Result<PtySpawnResult> {
    let pty_system = portable_pty::native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })?;
    let child = pair.slave.spawn_command(cmd)?;
    Ok(PtySpawnResult {
        master: pair.master,
        child,
        parent_fds_to_close: Vec::new(),
    })
}

