use riscv::register::{
    scause::{
        self,
        Trap,
        Exception,
        Interrupt
    },
    sepc,
    stvec,
    sscratch,
    sstatus
};
use crate::timer::{
    TICKS,
    clock_set_next_event
};
use crate::context::TrapFrame;
use crate::process::tick;
use crate::memory::access_pa_via_va;
use riscv::register::sie;

global_asm!(include_str!("trap/trap.asm"));

pub fn init() {
    unsafe {
        extern "C" {
            fn __alltraps();
        }
        sscratch::write(0);
        stvec::write(__alltraps as usize, stvec::TrapMode::Direct);
        // set timer
        sstatus::set_sie();

        // enable external interrupt
		sie::set_sext();

		// closed by OpenSBI, so we open them manually
		// see https://github.com/rcore-os/rCore/blob/54fddfbe1d402ac1fafd9d58a0bd4f6a8dd99ece/kernel/src/arch/riscv32/board/virt/mod.rs#L4
		init_external_interrupt();
		enable_serial_interrupt();
    }
    println!("++++ setup interrupt! ++++");
}

pub unsafe fn init_external_interrupt() {
    let HART0_S_MODE_INTERRUPT_ENABLES: *mut u32 = access_pa_via_va(0x0c00_2080) as *mut u32;
    const SERIAL: u32 = 0xa;
    HART0_S_MODE_INTERRUPT_ENABLES.write_volatile(1 << SERIAL);
}

pub unsafe fn enable_serial_interrupt() {
    let UART16550: *mut u8 = access_pa_via_va(0x10000000) as *mut u8;
    UART16550.add(4).write_volatile(0x0B);
    UART16550.add(1).write_volatile(0x01);
}

#[no_mangle]
pub fn rust_trap(tf: &mut TrapFrame) {
    // println!("rust_trap!");
    match tf.scause.cause() {
        Trap::Exception(Exception::Breakpoint) => breakpoint(&mut tf.sepc),
        Trap::Interrupt(Interrupt::SupervisorTimer) => super_timer(),
        Trap::Exception(Exception::InstructionPageFault) => page_fault(tf),
        Trap::Exception(Exception::LoadPageFault) => page_fault(tf),
        Trap::Exception(Exception::StorePageFault) => page_fault(tf),
		Trap::Exception(Exception::UserEnvCall) => syscall(tf),
		Trap::Interrupt(Interrupt::SupervisorExternal) => external(),
        _ => {
            panic!("undefined trap!")
            // let cause = scause::read().cause();
            // let epc = sepc::read();
            // println!("trap: cause: {:?}, epc: 0x{:#x}", cause, epc);
        }
    }
}

fn breakpoint(sepc: &mut usize) {
    println!("a breakpoint set @0x{:x}", sepc);
    *sepc += 2;
}

fn super_timer() {
    clock_set_next_event();
    // unsafe {
    //     TICKS += 1;
    //     if (TICKS == 100) {
    //         TICKS = 0;
    //         println!("* 100 ticks *");
    //     }
    // }
    tick();
}

fn page_fault(tf: &mut TrapFrame) {
    println!("{:?} va = {:#x} instruction = {:#x}", tf.scause.cause(), tf.stval, tf.sepc);
    panic!("page fault!");
}

fn syscall(tf: &mut TrapFrame) {
    tf.sepc += 4;
    let ret = crate::syscall::syscall(
        tf.x[17],
        [tf.x[10], tf.x[11], tf.x[12]],
        tf
    );
    tf.x[10] = ret as usize;
}

fn external() {
    // 键盘属于一种串口设备，而实际上有很多种外设
    // 这里我们只考虑串口
    let _ = try_serial();
}

fn try_serial() -> bool {
    // 通过 OpenSBI 获取串口输入
    match super::io::getchar_option() {
        Some(ch) => {
            // 将获取到的字符输入标准输入
            if (ch == '\r') {
                crate::fs::stdio::STDIN.push('\n');
            }
            else {
                crate::fs::stdio::STDIN.push(ch);
            }
            true
        },
        None => false
    }
}

#[inline(always)]
pub fn disable_and_store() -> usize {
    let sstatus: usize;
    unsafe {
        asm!("csrci sstatus, 1 << 1" : "=r"(sstatus) ::: "volatile");
    }
    sstatus
}

#[inline(always)]
pub fn restore(flags: usize) {
    unsafe {
        asm!("csrs sstatus, $0" :: "r"(flags) :: "volatile");
    }
}

#[inline(always)]
pub fn enable_and_wfi() {
    unsafe {
        asm!("csrsi sstatus, 1 << 1; wfi" :::: "volatile");
    }
}