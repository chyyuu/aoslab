//! 文件相关的内核功能

use super::*;
use crate::fs::*;
use core::slice::from_raw_parts_mut;

// pub const O_RDONLY: u32 = 0;
// pub const O_WRONLY: u32 = 1;
// pub const O_RDWR: u32 = 2;
pub const O_CREAT: u32 = 64;
// pub const O_APPEND: u32 = 1024;

/// 从指定的文件中读取字符
///
/// 如果缓冲区暂无数据，返回 0；出现错误返回 -1
// todo: inode 放到 process 中去
pub(super) fn sys_read(fd: usize, buffer: *mut u8, size: usize) -> SyscallResult {
    // 从线程中获取 inode，注意避免锁
    let inode: Arc<dyn INode> =
        if let Some(inode) = PROCESSOR.get().current_thread().inner().descriptors.get(fd) {
            inode.clone()
        } else {
            return SyscallResult::Proceed(-1);
        };
    let buffer = unsafe { from_raw_parts_mut(buffer, size) };
    if let Ok(ret) = inode.read_at(0, buffer) {
        let ret = ret as isize;
        if ret > 0 {
            return SyscallResult::Proceed(ret);
        } else if ret == 0 {
            return SyscallResult::Park(ret);
        }
    }
    SyscallResult::Proceed(-1)
}

/// 将字符写入指定的文件
pub(super) fn sys_write(fd: usize, buffer: *mut u8, size: usize) -> SyscallResult {
    if let Some(inode) = PROCESSOR.get().current_thread().inner().descriptors.get(fd) {
        let buffer = unsafe { from_raw_parts_mut(buffer, size) };
        if let Ok(ret) = inode.write_at(0, buffer) {
            let ret = ret as isize;
            if ret >= 0 {
                return SyscallResult::Proceed(ret);
            }
        }
    }
    SyscallResult::Proceed(-1)
}

pub unsafe fn from_cstr(s: *const u8) -> &'static str {
    use core::{ slice, str };
    let len = (0usize..).find(|&i| *s.add(i) == 0).unwrap();
    str::from_utf8(slice::from_raw_parts(s, len)).unwrap()
}

pub(super) fn sys_open(path: *const u8, flags: u32) -> SyscallResult {
    let thread = PROCESSOR.get().current_thread();
    let fd = thread.alloc_fd() as isize;
    let cstr_path = unsafe { from_cstr(path) };
    // println!("path {:?} flags in sys_open: {:?}", cstr_path, flags as u32);
    if flags & O_CREAT > 0 {
        ROOT_INODE
            .create(cstr_path, rcore_fs::vfs::FileType::File, 0o666)
            .expect("failed to create file");
        println!("created {:?}", cstr_path);
    }
    let inode = ROOT_INODE.lookup(cstr_path).unwrap().clone();

    thread.inner().descriptors.push(inode);

    SyscallResult::Proceed(fd)
}

pub(super) fn sys_close(fd: i32) -> SyscallResult {
    let thread = PROCESSOR.get().current_thread();
    assert!(fd < thread.inner().descriptors.len() as i32);
    thread.dealloc_fd(fd);

    SyscallResult::Proceed(0)
}