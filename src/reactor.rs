use crate::reactor::op::{Lifecycle, Op, Opable};

use io_uring::cqueue;

use slab::Slab;
use std::cell::RefCell;

use std::io;
use std::rc::Rc;
use std::task::{Context, Poll};

pub(crate) mod accept;
pub(crate) mod op;
pub(crate) mod read;
pub(crate) mod write;

#[inline]
pub(crate) fn get_reactor() -> Rc<RefCell<Reactor>> {
    crate::executor::EX.with(|ex| ex.reactor.clone())
}

pub(crate) struct Ops {
    lifecycle: Slab<Lifecycle>,
}

impl Ops {
    pub(crate) fn new() -> Self {
        const DEFAULT_CAPACITY: usize = 8;
        Self {
            lifecycle: Slab::with_capacity(DEFAULT_CAPACITY),
        }
    }

    pub(crate) fn complete(&mut self, index: usize, cqe: cqueue::Entry) {
        self.lifecycle[index].complete(cqe);
    }

    pub(crate) fn poll_op(&mut self, index: usize, cx: &mut Context<'_>) -> Poll<cqueue::Entry> {
        self.lifecycle[index].poll_op(cx)
    }

    pub(crate) fn insert(&mut self) -> usize {
        self.lifecycle.insert(Lifecycle::Submitted)
    }
}

pub struct Reactor {
    ops: Ops,
    uring: io_uring::IoUring,
}

impl Reactor {
    pub fn new(entries: u32) -> io::Result<Self> {
        Ok(Self {
            ops: Ops::new(),
            uring: io_uring::IoUring::new(entries)?,
        })
    }

    pub(crate) fn wait(&self) -> io::Result<usize> {
        println!("[reactor] waiting");
        self.uring.submit_and_wait(1)
    }

    pub(crate) fn submit(&mut self) -> io::Result<()> {
        loop {
            match self.uring.submit() {
                Ok(_) => {
                    self.uring.submission().sync();
                }
                Err(ref e) if e.raw_os_error() == Some(libc::EBUSY) => {
                    self.dispatch_completions();
                }
                Err(e) if e.raw_os_error() != Some(libc::EINTR) => return Err(e),
                _ => continue,
            }
        }
    }

    pub(crate) fn dispatch_completions(&mut self) {
        let cq = &mut self.uring.completion();
        cq.sync();
        for cqe in cq {
            if cqe.user_data() == u64::MAX {
                continue;
            }

            let index = cqe.user_data() as _;
            self.ops.complete(index, cqe);
        }
    }

    pub fn submit_op<T>(&mut self, mut data: T) -> io::Result<Op<T>>
    where
        T: Opable,
    {
        let index = self.ops.insert();
        let sqe = data.opcode().user_data(index as _);

        let op = Op::new(data, index);
        while unsafe { self.uring.submission().push(&sqe).is_err() } {
            self.submit()?;
        }
        Ok(op)
    }

    pub fn poll_op(&mut self, index: usize, cx: &mut Context<'_>) -> Poll<cqueue::Entry> {
        self.ops.poll_op(index, cx)
    }
}
