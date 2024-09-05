use crate::reactor::op::{Completable, CqeResult, Opable};
use io_uring::squeue::Entry;
use io_uring::{opcode, types};
use std::io;
use std::os::fd::{AsRawFd, RawFd};

use super::get_reactor;
use super::op::Op;

pub(crate) struct Write<'a> {
    fd: RawFd,
    buf: &'a [u8],
    offset:u64
}

impl Op<Write<'_>> {
    pub fn write_at(fd: RawFd, buf: &[u8],offset:u64) -> io::Result<Op<Write<'_>>> {
        let reactor = get_reactor();
        let op = reactor.borrow_mut().submit_op(Write {
            fd,
            buf,
            offset
        });
        op
    }
}

impl Opable for Write<'_> {
    fn opcode(&mut self) -> Entry {
        opcode::Write::new(
            types::Fd(self.fd.as_raw_fd()),
            self.buf.as_ptr(),
            self.buf.len() as _,
        )
        .offset(self.offset)
        .build()
    }
}

impl<'a> Completable for Write<'a> {
    type Output = (std::io::Result<usize>, &'a [u8]);

    fn complete(self, cqe: CqeResult) -> Self::Output {
        let res = cqe.result.map(|v| v as usize);
        let buf = self.buf;
        (res, buf)
    }
}
