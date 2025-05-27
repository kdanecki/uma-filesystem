use core::slice;
use std::{
    ffi::{CStr, CString},
    fs::OpenOptions,
};
mod fuse;

use memmap2::MmapMut;

use crate::types::*;

/// cbindgen:no-export
type fuse_fill_dir_t = unsafe extern "C" fn(
    buf: *mut ::std::os::raw::c_void,
    name: *const ::std::os::raw::c_char,
    stbuf: *const i8,
    off: ::std::os::raw::c_long,
) -> ::std::os::raw::c_int;

#[no_mangle]
pub unsafe extern "C" fn rs_getattr(
    fs: *mut FileSystem,
    filename: *const ::std::os::raw::c_char,
    inode_buf: *mut inode_t,
) -> i32 {
    //(*fs).test();
    if let Some(inode) = (*fs).get_attr(CStr::from_ptr(filename)) {
        *inode_buf = inode;
        return 0;
    }
    -1
}

#[no_mangle]
pub unsafe extern "C" fn rs_open(
    fs: *mut FileSystem,
    filename: *const ::std::os::raw::c_char,
) -> i32 {
    if (*fs).get_attr(CStr::from_ptr(filename)).is_some() {
        return 0;
    }
    -1
}

#[no_mangle]
pub unsafe extern "C" fn rs_read(
    fs: *mut FileSystem,
    filename: *const ::std::os::raw::c_char,
    buf: *mut i8,
    size: usize,
) -> i32 {
    if let Some(bytes) = (*fs).read_file(CStr::from_ptr(filename)) {
        let size = size.min(bytes.len());
        buf.copy_from(bytes.as_ptr() as *const i8, size);
        size as i32
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn rs_readdir(
    fs: *mut FileSystem,
    filename: *const ::std::os::raw::c_char,
    buf: *mut ::std::os::raw::c_void,
    filler: fuse_fill_dir_t,
) -> i32 {
    let files = (*fs).get_files_in_dir(CStr::from_ptr(filename));
    if files.len() > 0 {
        for f in files {
            let name = CString::new(f).unwrap();
            if filler(buf, name.as_ptr() as *const i8, 0 as *const i8, 0) != 0 {
                panic!("filler failed");
            }
        }
        return 0;
    }
    return -1;
}

#[no_mangle]
pub unsafe extern "C" fn rs_create(
    fs: *mut FileSystem,
    filename: *const ::std::os::raw::c_char,
) -> i32 {
    if (*fs).create_file(CStr::from_ptr(filename), &[]).is_ok() {
        return 0;
    }
    return -1;
}

// TODO offset
#[no_mangle]
pub unsafe extern "C" fn rs_write(
    fs: *mut FileSystem,
    filename: *const ::std::os::raw::c_char,
    content: *const ::std::os::raw::c_char,
    size: usize,
) -> i32 {
    let content: &[u8] = slice::from_raw_parts(content as *const u8, size);
    (*fs).write_file(CStr::from_ptr(filename), content);
    return size as i32;
}

#[no_mangle]
pub unsafe extern "C" fn rs_mkdir(
    fs: *mut FileSystem,
    filename: *const ::std::os::raw::c_char,
) -> i32 {
    if (*fs).create_directory(CStr::from_ptr(filename)).is_ok() {
        return 0;
    }
    return -1;
}

#[no_mangle]
pub unsafe extern "C" fn rs_unlink(
    fs: *mut FileSystem,
    filename: *const ::std::os::raw::c_char,
) -> i32 {
    if (*fs).unlink_file(CStr::from_ptr(filename)).is_ok() {
        return 0;
    }
    return -1;
}

#[no_mangle]
pub unsafe extern "C" fn rs_rmdir(
    fs: *mut FileSystem,
    filename: *const ::std::os::raw::c_char,
) -> i32 {
    if (*fs).unlink_dir(CStr::from_ptr(filename)).is_ok() {
        return 0;
    }
    return -1;
}

#[no_mangle]
pub unsafe extern "C" fn rs_truncate(
    fs: *mut FileSystem,
    filename: *const ::std::os::raw::c_char,
    size: usize,
) -> i32 {
    if (*fs).truncate(CStr::from_ptr(filename), size).is_ok() {
        return 0;
    }
    return -1;
}

#[no_mangle]
pub unsafe extern "C" fn rs_rename(
    fs: *mut FileSystem,
    from: *const ::std::os::raw::c_char,
    to: *const ::std::os::raw::c_char,
) -> i32 {
    if (*fs)
        .rename(CStr::from_ptr(from), CStr::from_ptr(to))
        .is_ok()
    {
        return 0;
    }
    return -1;
}

#[no_mangle]
pub extern "C" fn rs_init<'a>() -> *mut FileSystem<'a> {
    let block_size = 1024;
    let block_num = 16348;

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("foo")
        .expect("failed to open file");
    file.set_len(block_size * block_num).expect("OOM");
    let file = Box::into_raw(Box::new(file));
    let map = Box::new(unsafe { MmapMut::map_mut(&(*file)).expect("failed mmap") });
    let map = Box::into_raw(map);

    let mut f = unsafe { Box::new(FileSystem::new(&mut (*map)[..])) };
    // println!("{:?}", f);
    f.format(1024, 16384);
    f.dummy_data();
    Box::into_raw(f)
}
