use bytes::Bytes;
use futures::stream::Stream;
use std::sync::mpsc;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

pub struct ReceiverStream {
    pub receiver: mpsc::Receiver<Bytes>,
}

impl Stream for ReceiverStream {
    type Item = Result<Bytes, mpsc::RecvError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.receiver.try_recv() {
            Ok(msg) => Poll::Ready(Some(Ok(msg))),
            Err(mpsc::TryRecvError::Empty) => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(mpsc::TryRecvError::Disconnected) => Poll::Ready(None),
        }
    }
}
