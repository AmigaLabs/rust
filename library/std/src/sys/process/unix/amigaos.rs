use libc::{c_char, c_int, pid_t};

use super::common::*;
use crate::ffi::{CStr, CString};
use crate::io;
use crate::num::NonZero;
use crate::os::unix::ffi::OsStrExt;
use crate::process::StdioPipes;
use crate::ptr;
use crate::sys::cvt;

#[path = "unsupported/wait_status.rs"]
mod wait_status;
pub use wait_status::ExitStatus;

fn unsupported_err() -> io::Error {
    io::Error::UNSUPPORTED_PLATFORM
}

fn unsupported<T>() -> io::Result<T> {
    Err(unsupported_err())
}

/// `spawnvpe` only adds or overrides variables (it takes a delta environment),
/// so variables removed with `env_remove`/`env_clear` must be unset in this
/// process before the spawn. The child copies the environment when it is
/// created, so the originals can be restored as soon as `spawnvpe` returns.
struct EnvRestore(Vec<(CString, Option<CString>)>);

impl Drop for EnvRestore {
    fn drop(&mut self) {
        for (name, old) in self.0.drain(..) {
            unsafe {
                match old {
                    Some(value) => libc::setenv(name.as_ptr(), value.as_ptr(), 1),
                    None => libc::unsetenv(name.as_ptr()),
                };
            }
        }
    }
}

fn unset_for_spawn(cmd: &Command) -> EnvRestore {
    let mut names: Vec<Vec<u8>> = Vec::new();
    if cmd.get_env_clear() {
        names.extend(crate::env::vars_os().map(|(k, _)| k.as_bytes().to_vec()));
    }
    names.extend(
        cmd.get_envs()
            .filter(|(_, v)| v.is_none())
            .map(|(k, _)| k.as_bytes().to_vec()),
    );

    let mut saved = Vec::with_capacity(names.len());
    for name in names {
        let Ok(cname) = CString::new(name) else { continue };
        let old = unsafe { libc::getenv(cname.as_ptr()) };
        let old = if old.is_null() { None } else { Some(unsafe { CStr::from_ptr(old) }.to_owned()) };
        unsafe { libc::unsetenv(cname.as_ptr()) };
        saved.push((cname, old));
    }
    EnvRestore(saved)
}

////////////////////////////////////////////////////////////////////////////////
// Command
////////////////////////////////////////////////////////////////////////////////

impl Command {
    pub fn spawn(
        &mut self,
        default: Stdio,
        needs_stdin: bool,
    ) -> io::Result<(Process, StdioPipes)> {
        let envp = self.capture_env();
        let envp_ptr = envp.as_ref().map_or(ptr::null_mut(), |e| e.as_ptr() as *mut *mut c_char);
        let cwd = self.get_cwd().map_or(ptr::null(), |c| c.as_ptr());
        let (ours, theirs) = self.setup_io(default, needs_stdin)?;
        let fhin = theirs.stdin.fd().unwrap_or(0);
        let fhout = theirs.stdout.fd().unwrap_or(1);
        let fherr = theirs.stderr.fd().unwrap_or(2);
        let argv = self.get_argv().as_ptr();
        let _env = unset_for_spawn(self);
        let pid = cvt(unsafe {
            libc::spawnvpe(self.get_program_cstr().as_ptr(), argv, envp_ptr, cwd, fhin, fhout, fherr)
        })?;
        // The parent's copies of the child ends are closed when `theirs` is dropped here.
        Ok((Process { handle: pid }, ours))
    }

    pub fn exec(&mut self, _default: Stdio) -> io::Error {
        unsupported_err()
    }
}

/// Reads stdout and stderr until EOF, then waits for the child. EOF arrives when
/// the child exits and closes its ends of the pipes. stderr is read after stdout,
/// so a child that fills the stderr pipe while stdout is still open can block.
pub fn output(cmd: &mut Command) -> io::Result<(ExitStatus, Vec<u8>, Vec<u8>)> {
    let (mut process, mut pipes) = cmd.spawn(Stdio::MakePipe, false)?;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    if let Some(out) = pipes.stdout.take() {
        out.read_to_end(&mut stdout)?;
    }
    if let Some(err) = pipes.stderr.take() {
        err.read_to_end(&mut stderr)?;
    }
    let status = process.wait()?;
    Ok((status, stdout, stderr))
}

////////////////////////////////////////////////////////////////////////////////
// Processes
////////////////////////////////////////////////////////////////////////////////

pub struct Process {
    handle: pid_t,
}

impl Process {
    pub fn id(&self) -> u32 {
        self.handle as u32
    }

    /// clib4 `kill()` only delivers Ctrl-C for SIGTERM/SIGINT/SIGQUIT and
    /// silently ignores SIGKILL, so `kill` sends SIGTERM.
    pub fn kill(&self) -> io::Result<()> {
        cvt(unsafe { libc::kill(self.handle, libc::SIGTERM) })?;
        Ok(())
    }

    pub fn send_signal(&self, signal: i32) -> io::Result<()> {
        cvt(unsafe { libc::kill(self.handle, signal) })?;
        Ok(())
    }

    pub fn send_process_group_signal(&self, _signal: i32) -> io::Result<()> {
        unsupported()
    }

    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        let mut status: c_int = 0;
        cvt(unsafe { libc::waitpid(self.handle, &mut status, 0) })?;
        Ok(ExitStatus::from(status))
    }

    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        let mut status: c_int = 0;
        let pid = cvt(unsafe { libc::waitpid(self.handle, &mut status, libc::WNOHANG) })?;
        if pid == 0 { Ok(None) } else { Ok(Some(ExitStatus::from(status))) }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct ExitStatusError(NonZero<c_int>);

impl Into<ExitStatus> for ExitStatusError {
    fn into(self) -> ExitStatus {
        ExitStatus::from(c_int::from(self.0))
    }
}

impl ExitStatusError {
    pub fn code(self) -> Option<NonZero<i32>> {
        ExitStatus::from(c_int::from(self.0)).code().map(|st| st.try_into().unwrap())
    }
}
