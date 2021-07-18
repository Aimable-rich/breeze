use std::future::Future;
use std::io::Result;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

use ds::{ResponseRingBuffer, RingSlice};

use protocol::Protocol;

use tokio::io::{AsyncRead, ReadBuf};

use futures::ready;

pub trait ResponseHandler {
    fn load_offset(&self) -> usize;
    // 从backend接收到response，并且完成协议解析时调用
    fn on_received(&self, seq: usize, response: RingSlice);
}

unsafe impl<R, W, P> Send for BridgeResponseToLocal<R, W, P> {}
unsafe impl<R, W, P> Sync for BridgeResponseToLocal<R, W, P> {}

pub struct BridgeResponseToLocal<R, W, P> {
    seq: usize,
    done: Arc<AtomicBool>,
    r: R,
    w: W,
    parser: P,
    data: ResponseRingBuffer,
}

impl<R, W, P> BridgeResponseToLocal<R, W, P> {
    pub fn from(r: R, w: W, parser: P, buf: usize, done: Arc<AtomicBool>) -> Self {
        debug_assert!(buf == buf.next_power_of_two());
        Self {
            seq: 0,
            w: w,
            r: r,
            parser: parser,
            data: ResponseRingBuffer::with_capacity(buf),
            done: done,
        }
    }
}

impl<R, W, P> Future for BridgeResponseToLocal<R, W, P>
where
    R: AsyncRead + Unpin,
    P: Protocol + Unpin,
    W: ResponseHandler + Unpin,
{
    type Output = Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        log::debug!("resp-handler: task polling.");
        let me = &mut *self;
        let mut reader = Pin::new(&mut me.r);
        //let mut spins = 0;
        while !me.done.load(Ordering::Acquire) {
            let offset = me.w.load_offset();
            me.data.reset_read(offset);
            let mut buf = me.data.as_mut_bytes();
            log::debug!("resp-handler: {} bytes available oft:{}", buf.len(), offset);
            if buf.len() == 0 {
                log::info!(
                    "resp-handler: buffer full. read:{} processed:{} write:{}",
                    offset,
                    me.data.processed(),
                    me.data.writtened()
                );
                std::hint::spin_loop();
                continue;
            }
            let mut buf = ReadBuf::new(&mut buf);
            ready!(reader.as_mut().poll_read(cx, &mut buf))?;
            // 一共读取了n个字节
            let n = buf.capacity() - buf.remaining();
            log::debug!(
                "resp-handler:{} bytes read. data:{:?}",
                n,
                &buf.filled()[0..n.min(48)]
            );
            if n == 0 {
                break; // EOF
            }
            me.data.advance_write(n);
            // 处理等处理的数据
            while me.data.processed() < me.data.writtened() {
                let mut response = me.data.processing_bytes();
                let (found, num) = me.parser.parse_response(&response);
                log::debug!("resp-handler: parsed:{} num:{} seq:{} ", found, num, me.seq);
                if !found {
                    break;
                }
                response.resize(num);
                let seq = me.seq;
                me.seq += 1;

                me.w.on_received(seq, response);
                me.data.advance_processed(num);
            }
        }
        log::debug!("resp-handler: task of reading data from response complete");
        Poll::Ready(Ok(()))
    }
}
