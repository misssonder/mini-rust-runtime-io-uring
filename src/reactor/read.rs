use crate::reactor::op::{Completable, CqeResult, Opable};

use io_uring::squeue::Entry;
use io_uring::{opcode, types};
use std::io;
use std::os::fd::{AsRawFd, RawFd};

use super::get_reactor;
use super::op::Op;

pub(crate) struct Read<'a> {
    fd: RawFd,
    buf: &'a mut [u8],
    offset:u64
}

impl Op<Read<'_>> {
    pub fn read_at(fd: RawFd, buf: &mut [u8],offset :u64) -> io::Result<Op<Read<'_>>> {
        let reactor = get_reactor();
        let op = reactor.borrow_mut().submit_op(Read {
            fd,
            buf,
            offset,
        });
        op
    }
}

impl Opable for Read<'_> {
    fn opcode(&mut self) -> Entry {
        opcode::Read::new(
            types::Fd(self.fd.as_raw_fd()),
            self.buf.as_mut_ptr(),
            self.buf.len() as _,
        )
        .offset(self.offset)
        .build()
    }
}

impl<'a> Completable for Read<'a> {
    type Output = (std::io::Result<usize>, &'a mut [u8]);

    fn complete(self, cqe: CqeResult) -> Self::Output {
        let res = cqe.result.map(|v| v as usize);
        let buf = self.buf;
        (res, buf)
    }
}
