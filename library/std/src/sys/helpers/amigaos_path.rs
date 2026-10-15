//! AmigaOS path rewriting for clib4.
//!
//! clib4 takes AmigaDOS names ("Volume:dir/file") and, in unix path mode, unix
//! names ("/dir/file"). The rewrites below are needed in both modes: "." is not
//! an AmigaDOS name, and a unix absolute path such as "/.git" names the volume
//! ".git:", which makes AmigaDOS ask for that disk.

use crate::borrow::Cow;
use crate::ffi::CStr;
use crate::io;
use crate::vec::Vec;

/// The current directory as clib4 reports it. In unix path mode that is a unix
/// path such as "/Work/dir".
pub fn raw_cwd() -> io::Result<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::with_capacity(512);
    loop {
        unsafe {
            let ptr = buf.as_mut_ptr() as *mut libc::c_char;
            if !libc::getcwd(ptr, buf.capacity()).is_null() {
                let len = CStr::from_ptr(ptr).to_bytes().len();
                buf.set_len(len);
                return Ok(buf);
            }
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ERANGE) {
            return Err(error);
        }
        buf.reserve(buf.capacity());
    }
}

/// Rewrites a path into the form clib4 takes in both path modes:
///
/// - "." and "./" become the current directory;
/// - empty components and "." components are dropped;
/// - ".." removes the component before it; a relative path that starts with
///   ".." starts from the current directory, since AmigaDOS has no "..";
/// - an absolute unix path "/Volume/dir" becomes "Volume:dir".
pub fn to_amiga_bytes(path: &[u8]) -> Cow<'_, [u8]> {
    let absolute = path.first() == Some(&b'/');
    if path.is_empty() || (absolute && path.len() == 1) {
        return Cow::Borrowed(path);
    }

    let parts: Vec<&[u8]> =
        path.split(|&b| b == b'/').filter(|part| !part.is_empty() && *part != b".").collect();
    if parts.is_empty() {
        return match raw_cwd() {
            Ok(cwd) => Cow::Owned(to_amiga_bytes(&cwd).into_owned()),
            Err(_) => Cow::Borrowed(path),
        };
    }

    // Components are collected first so that ".." can remove the one before it.
    let mut volume: Option<Vec<u8>> = None;
    let mut segs: Vec<Vec<u8>> = Vec::new();
    let mut tail: &[&[u8]] = &parts;

    if absolute {
        // "/Volume/dir" names the volume "Volume".
        volume = Some(parts[0].to_vec());
        tail = &parts[1..];
    } else if let Some(colon) = parts[0].iter().position(|&b| b == b':') {
        // "Volume:dir/file" names the volume explicitly.
        volume = Some(parts[0][..colon].to_vec());
        if colon + 1 < parts[0].len() {
            segs.push(parts[0][colon + 1..].to_vec());
        }
        tail = &parts[1..];
    } else if parts[0] == &b".."[..] {
        // A relative path that climbs up starts from the current directory.
        let cwd = match raw_cwd() {
            Ok(cwd) => to_amiga_bytes(&cwd).into_owned(),
            Err(_) => Vec::new(),
        };
        (volume, segs) = split_volume(&cwd);
    }

    for part in tail {
        let part: &[u8] = part;
        if part == &b".."[..] {
            match segs.last() {
                Some(last) if last.as_slice() != &b".."[..] => {
                    segs.pop();
                }
                _ => segs.push(part.to_vec()),
            }
        } else {
            segs.push(part.to_vec());
        }
    }

    let mut out: Vec<u8> = Vec::with_capacity(path.len());
    if let Some(volume) = &volume {
        out.extend_from_slice(volume);
        out.push(b':');
    }
    for (i, seg) in segs.iter().enumerate() {
        if i > 0 {
            out.push(b'/');
        }
        out.extend_from_slice(seg);
    }

    if out == path { Cow::Borrowed(path) } else { Cow::Owned(out) }
}

/// Splits an AmigaDOS path into its volume and directory components.
fn split_volume(path: &[u8]) -> (Option<Vec<u8>>, Vec<Vec<u8>>) {
    let (volume, rest) = match path.iter().position(|&b| b == b':') {
        Some(colon) => (Some(path[..colon].to_vec()), &path[colon + 1..]),
        None => (None, path),
    };
    let segs: Vec<Vec<u8>> =
        rest.split(|&b| b == b'/').filter(|seg| !seg.is_empty()).map(|seg| seg.to_vec()).collect();
    (volume, segs)
}
