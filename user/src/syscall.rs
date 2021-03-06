//! 系统调用

pub const STDIN: usize = 0;
pub const STDOUT: usize = 1;

const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_OPEN: usize = 56;
const SYSCALL_CLOSE: usize = 57;
const SYSCALL_EXEC: usize = 221;

/// 将参数放在对应寄存器中，并执行 `ecall`
fn syscall(id: usize, arg0: usize, arg1: usize, arg2: usize) -> isize {
    // 返回值
    let mut ret;
    unsafe {
        llvm_asm!("ecall"
            : "={x10}" (ret)
            : "{x10}" (arg0), "{x11}" (arg1), "{x12}" (arg2), "{x17}" (id)
            : "memory"      // 如果汇编可能改变内存，则需要加入 memory 选项
            : "volatile"); // 防止编译器做激进的优化（如调换指令顺序等破坏 SBI 调用行为的优化）
    }
    ret
}

// /// 读取字符
// pub fn sys_read(fd: usize, buffer: &mut [u8]) -> isize {
//     loop {
//         let ret = syscall(SYSCALL_READ, fd, buffer as *const [u8] as *const u8 as usize, buffer.len());
//         if ret > 0 {
//             return ret;
//         }
//     }
// }

pub fn sys_read(fd: usize, buffer: &mut [u8], len: usize) -> isize {
    syscall(SYSCALL_READ, fd, buffer as *const [u8] as *const u8 as usize, len)
}

/// 打印字符串
pub fn sys_write(fd: usize, buffer: &[u8], len: usize) -> isize {
    syscall(SYSCALL_WRITE, fd, buffer as *const [u8] as *const u8 as usize, len)
}

/// 退出并返回数值
pub fn sys_exit(code: isize) -> ! {
    syscall(SYSCALL_EXIT, code as usize, 0, 0);
    unreachable!()
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    syscall(SYSCALL_OPEN, path as usize, flags as usize, 0)
}

pub fn sys_close(fd: i32) -> isize {
    syscall(SYSCALL_CLOSE, fd as usize, 0, 0)
}

pub fn sys_exec(path: *const u8) {
    syscall(SYSCALL_EXEC, path as usize, 0, 0);
}