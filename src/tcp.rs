use std::{
    io::{self},
    net::{SocketAddr, ToSocketAddrs},
    os::{fd::RawFd, unix::prelude::AsRawFd},
};



use socket2::{Domain, Protocol, Socket, Type};


use crate::reactor::{op::Op};

#[derive(Debug)]
pub struct TcpStream {
    fd: RawFd,
}

impl TcpStream {
    pub(crate) fn new(fd: RawFd) -> Self {
        Self { fd }
    }
}

pub struct TcpListener {
    listener: std::net::TcpListener,
}

impl TcpListener {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        let addr = addr
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "empty address"))?;
        let domain = if addr.is_ipv6() {
            Domain::IPV6
        } else {
            Domain::IPV4
        };
        let sk = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))?;
        let addr = socket2::SockAddr::from(addr);
        sk.set_reuse_address(true)?;
        sk.set_reuse_port(true)?;
        sk.bind(&addr)?;
        sk.listen(1024)?;

        println!("tcp bind with fd {}", sk.as_raw_fd());
        Ok(Self {
            listener: sk.into(),
        })
    }

    pub async fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        let accept = Op::accept(self.listener.as_raw_fd())?;
        accept.await
    }
}

impl TcpStream {
    pub async fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        let fd = self.fd.as_raw_fd();
        let read = Op::read_at(fd, buf, 0)?;
        read.await.0
    }

    pub async fn write(&self, buf: &[u8]) -> io::Result<usize> {
        let fd = self.fd.as_raw_fd();
        let write = Op::write_at(fd, buf, 0)?;
        write.await.0
    }
}
