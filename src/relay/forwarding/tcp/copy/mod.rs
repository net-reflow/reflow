use std::io;
use std::time;

use futures::task::Context;
use futures::{Future, Poll as Poll1};
use futures_timer::Delay;
use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncWrite};

/// A future which will copy all data from a reader into a writer.
/// modified version of Copy from tokio
/// prints more verbose logs
pub struct CopyVerboseTime<R, W> {
    reader: Option<R>,
    read_done: bool,
    writer: Option<W>,
    pos: usize,
    cap: usize,
    amt: u64,
    timeout: time::Duration,
    timer: Option<Delay>,
    buf: Box<[u8]>,
}

#[derive(Debug, Fail)]
pub enum CopyError {
    #[fail(display = "poll_read: {}", err)]
    ReadError { err: io::Error },
    #[fail(display = "read nothing in a long time")]
    ReadTimeout,
    #[fail(display = "poll_write: {}", err)]
    WriteError { err: io::Error },
    #[fail(display = "wrote zero bytes")]
    WriteZero,
    #[fail(display = "poll_flush: {}", err)]
    FlushError { err: io::Error },
}

impl CopyError {
    pub fn is_read(&self) -> bool {
        match self {
            CopyError::ReadError { .. } => true,
            CopyError::ReadTimeout => true,
            _ => false,
        }
    }
}

/// Creates a future which represents copying all the bytes from one object to
/// another.
///
/// The returned future will copy all the bytes read from `reader` into the
/// `writer` specified. This future will only complete once the `reader` has hit
/// EOF and all bytes have been written to and flushed from the `writer`
/// provided.
///
/// On success the number of bytes is returned and the `reader` and `writer` are
/// consumed. On error the error is returned and the I/O objects are consumed as
/// well.
pub fn copy_verbose<R, W>(reader: R, writer: W) -> CopyVerboseTime<R, W>
where
    R: AsyncRead,
    W: AsyncWrite,
{
    CopyVerboseTime {
        reader: Some(reader),
        read_done: false,
        writer: Some(writer),
        amt: 0,
        pos: 0,
        cap: 0,
        timeout: time::Duration::from_secs(500),
        timer: None,
        buf: Box::new([0; 2048]),
    }
}

impl<R, W> Future for CopyVerboseTime<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    type Output = Result<(u64, R, W), CopyError>;

    fn poll(
        mut self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
    ) -> Poll1<Result<(u64, R, W), CopyError>> {
        loop {
            // If our buffer is empty, then we need to read some data to
            // continue.
            if self.pos == self.cap && !self.read_done {
                let me = &mut *self;
                let r = Pin::new(me.reader.as_mut().unwrap())
                    .poll_read(ctx, &mut me.buf)
                    .map_err(|e| CopyError::ReadError { err: e })?;
                let n = match r {
                    Poll1::Ready(x) => {
                        self.clear_timer();
                        x
                    }
                    Poll1::Pending => {
                        self.test_timeout(ctx)?;
                        return Poll1::Pending;
                    }
                };
                if n == 0 {
                    self.read_done = true;
                } else {
                    self.pos = 0;
                    self.cap = n;
                }
            }

            // If our buffer has some data, let's write it out!
            while self.pos < self.cap {
                let me = &mut *self;
                let w = Pin::new(me.writer.as_mut().unwrap())
                    .poll_write(ctx, &mut me.buf[me.pos..me.cap])
                    .map_err(|e| CopyError::WriteError { err: e })?;

                let i = match w {
                    Poll1::Ready(x) => x,
                    Poll1::Pending => {
                        return Poll1::Pending;
                    }
                };
                if i == 0 {
                    return Poll1::Ready(Err(CopyError::WriteZero));
                } else {
                    self.pos += i;
                    self.amt += i as u64;
                }
            }

            // If we've written al the data and we've seen EOF, flush out the
            // data and finish the transfer.
            // done with the entire transfer.
            if self.pos == self.cap && self.read_done {
                let me = &mut *self;
                let w = Pin::new(me.writer.as_mut().unwrap())
                    .poll_flush(ctx)
                    .map_err(|e| CopyError::FlushError { err: e })?;
                match w {
                    Poll1::Ready(_) => {}
                    Poll1::Pending => {
                        return Poll1::Pending;
                    }
                }
                let reader = self.reader.take().unwrap();
                let writer = self.writer.take().unwrap();
                return Poll1::Ready(Ok((self.amt, reader, writer)));
            }
        }
    }
}

impl<R, W> CopyVerboseTime<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    fn test_timeout(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Result<(), CopyError> {
        let me = &mut *self;
        if me.timer.is_none() {
            let d = Delay::new(me.timeout);
            me.timer = Some(d);
        }
        let t = Pin::new(me.timer.as_mut().unwrap()).poll(cx);
        match t {
            Poll1::Ready(()) => return Err(CopyError::ReadTimeout),
            Poll1::Pending => {}
        }
        Ok(())
    }

    fn clear_timer(&mut self) {
        let _ = self.timer.take();
    }
}
