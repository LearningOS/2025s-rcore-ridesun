//! Process management syscalls

use core::ptr::addr_of;
use crate::mm::{translated_byte_buffer, PageTable, PhysAddr, VirtAddr};
use crate::task::{change_program_brk, current_user_token, exit_current_and_run_next, get_syscall_times, mmap_cur_task, munmap_cur_task, suspend_current_and_run_next};
use crate::timer::get_time_us;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let buffers =translated_byte_buffer(current_user_token(), ts as *const u8, core::mem::size_of::<TimeVal>());
    let us = get_time_us();
    let tv= TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    let mut tv_ptr =addr_of!(tv) as *const u8;
    for buffer in buffers{
        unsafe {
            tv_ptr.copy_to(buffer.as_mut_ptr(),buffer.len());
            tv_ptr=tv_ptr.offset(buffer.len() as isize);
        }
    }
    0
}

/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    let page_table = PageTable::from_token(current_user_token());
    let va = VirtAddr::from(id);
    let pte = match page_table.translate(va.floor()) {
        Some(pte) => pte,
        None => return -1,
    };
    let pa: PhysAddr = pte.ppn().into();
    let addr = pa.0 | va.page_offset();
    match trace_request {
        0 => {
            debug!("read id");
            if pte.is_user() && pte.readable() {
                let raw_ptr = addr as *const u8;
                unsafe { *raw_ptr as isize }
            } else {
                -1
            }
        },
        1 => {
            debug!("write data");
            if pte.is_user() && pte.writable() {
                let raw_ptr = addr as *mut u8;
                unsafe {
                    *raw_ptr = data as u8;
                    0
                }
            } else {
                -1
            }
        },
        2 => {
            get_syscall_times(id) as isize
        },
        _ => {-1}
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    match mmap_cur_task(start,len,prot) {
        Ok(_) => 0,
        Err(e) => {error!("{e}");-1}
    }
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    match munmap_cur_task(start,len) {
        Ok(_) => 0,
        Err(e) => {error!("{e}");-1}
    }
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
