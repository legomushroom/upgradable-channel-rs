use std::{task::{Poll, Context}, pin::Pin};

use anyhow::Result;
use tokio::io::ReadBuf;

pub trait AsyncRead {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<()>>;
}
