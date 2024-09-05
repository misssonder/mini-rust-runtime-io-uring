use futures::ready;
use io_uring::{cqueue, squeue};
use std::future::{ Future};
use std::task::{Context, Poll, Waker};
use std::{io, mem};

use super::get_reactor;


#[derive(Debug)]
pub(crate) struct CqeResult {
    pub(crate) result: io::Result<u32>,
    #[allow(dead_code)]
    pub(crate) flags: u32,
}

impl From<cqueue::Entry> for CqeResult {
    fn from(cqe: cqueue::Entry) -> Self {
        let result = cqe.result();
        let flags = cqe.flags();
        let result = if result >= 0 {
            Ok(result as u32)
        } else {
            Err(io::Error::from_raw_os_error(-result))
        };
        CqeResult { result, flags }
    }
}

pub trait Completable {
    type Output;
    fn complete(self, cqe: CqeResult) -> Self::Output;
}

pub trait Opable {
    fn opcode(&mut self) -> squeue::Entry;
}

#[derive(Debug)]
pub(crate) enum Lifecycle {
    Submitted,
    Waiting(Waker),
    Complete(cqueue::Entry),
}

impl Lifecycle {
    pub(crate) fn complete(&mut self, cqe: cqueue::Entry) {
        match mem::replace(self, Lifecycle::Submitted) {
            x @ Lifecycle::Submitted | x @ Lifecycle::Waiting(..) => {
                if let Lifecycle::Waiting(waker) = x {
                    waker.wake()
                }
                *self = Lifecycle::Complete(cqe)
            }
            Lifecycle::Complete(_) => unreachable!(),
        }
    }

    pub(crate) fn poll_op(&mut self, cx: &mut Context<'_>) -> Poll<cqueue::Entry> {
        match mem::replace(self, Lifecycle::Submitted) {
            Lifecycle::Submitted => {
                *self = Lifecycle::Waiting(cx.waker().clone());
                Poll::Pending
            }
            Lifecycle::Waiting(waker) if !waker.will_wake(cx.waker()) => {
                *self = Lifecycle::Waiting(cx.waker().clone());
                Poll::Pending
            }
            Lifecycle::Waiting(waker) => {
                *self = Lifecycle::Waiting(waker);
                Poll::Pending
            }
            Lifecycle::Complete(cqe) => Poll::Ready(cqe),
        }
    }
}

pub(crate) struct Op<T> {
    pub(crate) data: Option<T>,
    pub(crate) index: usize,
}

impl<T> Op<T> {
    pub(crate) fn new(data: T, index: usize) -> Op<T> {
        Self { data:Some(data), index }
    }
}

impl<T> Future for Op<T>
where
    T: Completable + Unpin,
{
    type Output = T::Output;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        let reactor = get_reactor();
        let cqe = ready!(reactor.borrow_mut().poll_op(this.index, cx));
        Poll::Ready(this.data.take().unwrap().complete(CqeResult::from(cqe)))
    }
}