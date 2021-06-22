use std::io::Result;
use std::pin::Pin;
use std::task::{Context, Poll};

use discovery::ServiceDiscover;

use cacheservice::Topology;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

macro_rules! define_endpoint {
    ($($top:tt, $item:ident, $type_name:tt, $ep:expr),+) => {
        pub enum Endpoint<D> {
            $($item($type_name<D>)),+
        }

        impl<D> Endpoint<D> where D: ServiceDiscover<Topology> + Unpin + 'static {
            pub fn from_discovery(name: &str, discovery:D) -> Result<Self> {
                match name {
                    $($ep => Ok(Self::$item($type_name::<D>::from_discovery(discovery)?)),)+
                    _ => panic!("not supported endpoint name"),
                }
            }
        }

        impl<D> AsyncRead for Endpoint<D> where D: Unpin{
            fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context, buf: &mut ReadBuf) -> Poll<Result<()>> {
                match &mut *self {
                    $(Self::$item(ref mut p) => Pin::new(p).poll_read(cx, buf),)+
                }
            }
        }

        impl<D> AsyncWrite for Endpoint<D> where D:Unpin{
            fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize>>{
                match &mut *self {
                    $(Self::$item(ref mut p) => Pin::new(p).poll_write(cx, buf),)+
                }
            }
            fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<()>> {
                match &mut *self {
                    $(Self::$item(ref mut p) => Pin::new(p).poll_flush(cx),)+
                }
            }
            fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<()>> {
                match &mut *self {
                    $(Self::$item(ref mut p) => Pin::new(p).poll_shutdown(cx),)+
                }
            }
        }
    };
}

mod cacheservice;
//mod pipe;

use cacheservice::CacheService;
//use pipe::Pipe;

define_endpoint! {
//    Pipe,         Pipe,         "pipe",
    Topology, CacheService, CacheService, "cs"
}
