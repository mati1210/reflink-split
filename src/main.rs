// SPDX-License-Identifier: MPL-2.0

use std::{ffi::OsString, fmt, fs, os::unix::ffi::OsStringExt, path::PathBuf};

use byte_unit::Byte;
use nix::fcntl::copy_file_range;

struct EmptyError();
impl fmt::Display for EmptyError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

macro_rules! die {
    ( $ty:block  $($msg:tt)*) => {
        match $ty {
            Ok(e) => e,
            Err(e) => {::log::error!("{} {e}", format_args!($($msg)*)); ::std::process::exit(1) }
        }
    };
}

fn main() {
    env_logger::init();

    let mut args = std::env::args_os().skip(1);

    let split_size = die!( {Byte::parse_str(
            die!({args.next().ok_or(EmptyError())} "no size argument!").to_str().unwrap() ,
            true) } "failed to parse size!")
    .as_u64();

    let mut path = PathBuf::from(die!( { args.next().ok_or(EmptyError()) } "no path argument!" ));
    let original = die!( { fs::File::open(&path) } "failed to open {}!",path.display());
    let mut original_size = original.metadata().map(|meta| meta.len()).unwrap();

    let mut files = 1u64;
    let mut buf = itoa::Buffer::new();
    loop {
        let ext = buf.format(files);
        path.as_mut_os_string().push(ext);

        let splitf = die!( {fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
        } "failed to create {}!", path.display());

        // not sure if this is the correct way to do this but it works, for now
        let mut to_write = split_size as usize;
        loop {
            let written = die!({copy_file_range(&original,None,&splitf,None,to_write)} "failed to write to {}!", path.display());

            if written == 0 {
                break;
            }

            to_write = match to_write.checked_sub(written) {
                Some(f) if f != 0 => f,
                _ => break,
            };
        }

        original_size = match original_size.checked_sub(split_size) {
            Some(f) if f != 0 => f,
            _ => break,
        };

        files += 1;
        let mut vec = path.into_os_string().into_vec();
        vec.truncate(vec.len() - ext.len());
        path = PathBuf::from(OsString::from_vec(vec));
    }
}
