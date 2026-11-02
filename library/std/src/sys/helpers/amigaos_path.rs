//! AmigaOS path rewriting for clib4.
//!
//! clib4 takes AmigaDOS names ("Volume:dir/file") and, in unix path mode, unix
//! names ("/dir/file"). The rewrites below are needed in both modes: "." is not
//! an AmigaDOS name, and a unix absolute path such as "/.git" names the volume
//! ".git:", which makes AmigaDOS ask for that disk.

use crate::borrow::Cow;
use crate::ffi::CStr;
use crate::io;
use crate::string::String;
use crate::sync::OnceLock;
use crate::vec::Vec;

// AmigaDOS names are 8-bit, in the character set of the Locale (ISO-8859-1 in western Europe,
// ISO-8859-2 for Polish or Czech, ISO-8859-5 for Russian...), while Rust paths are UTF-8.
// Paths are converted where they cross into clib4 (`to_native`) and back (`from_native`), so
// "año.docx" opens whatever the Locale. The table comes from `amiga_charset_table` when the
// program links it (amiga-gl reads the Locale's charset through diskfont.library); otherwise,
// and when it finds none, it is ISO-8859-1.

/// The Unicode code point of each byte of the system charset (0: undefined, except NUL).
fn charset() -> &'static [u32; 256] {
    static TABLE: OnceLock<[u32; 256]> = OnceLock::new();
    TABLE.get_or_init(|| {
        unsafe extern "C" {
            #[linkage = "extern_weak"]
            static amiga_charset_table: Option<unsafe extern "C" fn(*mut u32) -> u32>;
        }
        let mut table = [0u32; 256];
        for (i, slot) in table.iter_mut().enumerate() {
            *slot = i as u32;
        }
        // SAFETY: a weak reference is `None` when nothing defines the symbol; the function
        // writes at most 256 values.
        if let Some(read) = unsafe { amiga_charset_table } {
            let mut found = [0u32; 256];
            if unsafe { read(found.as_mut_ptr()) } != 0 {
                // ASCII is the same in every AmigaOS charset whatever the table says.
                for (i, slot) in table.iter_mut().enumerate().skip(0x80) {
                    *slot = found[i];
                }
            }
        }
        table
    })
}

/// A UTF-8 path as AmigaDOS bytes in the system charset. Bytes that aren't UTF-8 are taken to
/// be native already and pass unchanged; a character the charset lacks becomes '?'.
pub fn to_native(path: &[u8]) -> Cow<'_, [u8]> {
    if path.is_ascii() {
        return Cow::Borrowed(path);
    }
    let Ok(text) = crate::str::from_utf8(path) else { return Cow::Borrowed(path) };
    let table = charset();
    let mut out = Vec::with_capacity(path.len());
    for c in text.chars() {
        if c.is_ascii() {
            out.push(c as u8);
        } else {
            let code = c as u32;
            out.push((0x80..=0xFFusize).find(|&b| table[b] == code).map_or(b'?', |b| b as u8));
        }
    }
    Cow::Owned(out)
}

/// AmigaDOS bytes in the system charset as a UTF-8 path. A name with a byte the charset leaves
/// undefined is kept as it is, so that it can still be opened.
pub fn from_native(native: &[u8]) -> Cow<'_, [u8]> {
    if native.is_ascii() {
        return Cow::Borrowed(native);
    }
    let table = charset();
    let mut out = String::with_capacity(native.len() + 8);
    for &b in native {
        let c = if b < 0x80 { Some(b as char) } else { char::from_u32(table[b as usize]).filter(|&c| c != '\0') };
        match c {
            Some(c) => out.push(c),
            None => return Cow::Borrowed(native),
        }
    }
    Cow::Owned(out.into_bytes())
}

/// The current directory as a UTF-8 path (clib4's, converted from the system charset).
pub fn cwd() -> io::Result<Vec<u8>> {
    raw_cwd().map(|cwd| from_native(&cwd).into_owned())
}

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

/// Rewrites a UTF-8 path into the form clib4 takes in both path modes (still UTF-8: see
/// [`to_native`] for the bytes that go to clib4):
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
        return match cwd() {
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
        let cwd = match cwd() {
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
