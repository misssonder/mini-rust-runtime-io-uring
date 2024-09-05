use crate::reactor::op::{Completable, CqeResult, Op, Opable};
use crate::tcp::TcpStream;

use io_uring::squeue::Entry;
use io_uring::{opcode, types};
use std::io;
use std::net::SocketAddr;
use std::os::fd::{AsRawFd, RawFd};

use super::get_reactor;

pub(crate) struct Accept {
    fd: RawFd,
    pub(crate) socketaddr: Box<(libc::sockaddr_storage, libc::socklen_t)>,
}

impl Op<Accept> {
    pub fn accept(fd: RawFd) -> io::Result<Self> {
        let socketaddr = Box::new((
            unsafe { std::mem::zeroed() },
            std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t,
        ));
        let reactor = get_reactor();
        let op = reactor.borrow_mut().submit_op(Accept {
            fd,
            socketaddr,
        });
        op
    }
}

impl Opable for Accept {
    fn opcode(&mut self) -> Entry {
        opcode::Accept::new(
            types::Fd(self.fd.as_raw_fd()),
            &mut self.socketaddr.0 as *mut _ as *mut _,
            &mut self.socketaddr.1,
        )
        .flags(libc::O_CLOEXEC)
        .build()
    }
}

impl Completable for Accept {
    type Output = io::Result<(TcpStream, SocketAddr)>;

    fn complete(self, cqe: CqeResult) -> Self::Output {
        let fd = cqe.result?;
        let stream = TcpStream::new(fd as _);
        let (_, addr) = unsafe {
            socket2::SockAddr::try_init(move |addr_storage, len| {
                self.socketaddr.0.clone_into(&mut *addr_storage);
                *len = self.socketaddr.1;
                Ok(())
            })?
        };
        Ok((stream, addr.as_socket().unwrap()))
    }
}
