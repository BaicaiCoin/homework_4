use std::{
    cell::RefCell,
    io::{self, Read, Write},
    net::{SocketAddr, TcpListener as StdTcpListener, TcpStream as StdTcpStream, ToSocketAddrs},
    os::unix::prelude::AsRawFd,
    rc::{Rc, Weak},
    task::Poll,
};
use futures::Stream;
use socket2::{Domain, Protocol, Socket, Type};
use crate::{reactor::get_reactor, reactor::Reactor};

pub struct TcpStream {
    stream: StdTcpStream,
}

impl From<StdTcpStream> for TcpStream {
    fn from(stream: StdTcpStream) -> Self {
        let reactor = get_reactor();
        reactor.borrow_mut().add(stream.as_raw_fd());
        Self { stream }
    }
}

impl Drop for TcpStream {
    fn drop(&mut self) {
        println!("drop");
        let reactor = get_reactor();
        reactor.borrow_mut().delete(self.stream.as_raw_fd());
    }
}

impl tokio::io::AsyncRead for TcpStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let fd = self.stream.as_raw_fd();
        unsafe {
            let b = &mut *(buf.unfilled_mut() as *mut [std::mem::MaybeUninit<u8>] as *mut [u8]);
            println!("read for fd {}", fd);
            match self.stream.read(b) {
                Ok(n) => {
                    println!("read for fd {} done, {}", fd, n);
                    buf.assume_init(n);
                    buf.advance(n);
                    Poll::Ready(Ok(()))
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    println!("read for fd {} done WouldBlock", fd);
                    let reactor = get_reactor();
                    reactor
                        .borrow_mut()
                        .modify_readable(self.stream.as_raw_fd(), cx);
                    Poll::Pending
                }
                Err(e) => {
                    println!("read for fd {} done err", fd);
                    Poll::Ready(Err(e))
                }
            }
        }
    }
}