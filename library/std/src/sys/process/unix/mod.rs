#[cfg_attr(
    any(
        target_os = "espidf",
        target_os = "horizon",
        target_os = "nuttx",
        target_os = "l4re",
        target_os = "amigaos"
    ),
    allow(unused)
)]
mod common;

cfg_select! {
    target_os = "fuchsia" => {
        mod fuchsia;
        use fuchsia as imp;
    }
    target_os = "vxworks" => {
        mod vxworks;
        use vxworks as imp;
    }
    any(
        target_os = "espidf",
        target_os = "horizon",
        target_os = "vita",
        target_os = "nuttx",
        target_os = "l4re"
    ) => {
        mod unsupported;
        use unsupported as imp;
        pub use unsupported::output;
    }
    target_os = "amigaos" => {
        mod amigaos;
        use amigaos as imp;
        pub use amigaos::output;
    }
    _ => {
        mod unix;
        use unix as imp;
    }
}

#[cfg(target_os = "linux")]
mod pidfd;

pub use imp::{ExitStatus, ExitStatusError, Process};
#[cfg(target_os = "linux")]
pub use pidfd::PidFd;

pub use self::common::{
    ChildPipe, Command, CommandArgs, ExitCode, Stdio, getpid, getppid, read_output,
};
pub use crate::ffi::OsString as EnvKey;
