use std::pin::Pin;

use futures_util::Stream;
use orion::aead::streaming;
use tokio::{io::AsyncReadExt, sync::mpsc};

pub fn read_into_stream<'a, R>(
    reader: R,
    read_chunk_size: u32,
    mut sealer: Option<streaming::StreamSealer>,
    ticks_updater: Option<mpsc::UnboundedSender<usize>>,
) -> Pin<Box<impl Stream<Item = anyhow::Result<Vec<u8>>> + Send + 'static>>
where
    R: AsyncReadExt + Send + Unpin + 'static,
{
    let stream = async_stream::try_stream! {
        let mut reader = reader;

        let mut buf: Vec<u8> = vec![0; read_chunk_size as usize];
        loop {
            let mut is_last_chunk = false;
            let mut b_read = 0;
            while b_read < buf.len() {
                let n = reader.read(&mut buf[b_read..]).await?;
                b_read += n;

                if n == 0 {
                    is_last_chunk = true;
                    break;
                }
            }
            if b_read == 0 {
                break;
            }
            if is_last_chunk {
                buf.truncate(b_read);
            }

            if let Some(c) = ticks_updater.as_ref() {
                _ = c.send(b_read);
            }
            yield match &mut sealer {
                Some(s) => s.seal_chunk(&buf, match is_last_chunk {
                    true => &streaming::StreamTag::Finish,
                    false => &streaming::StreamTag::Message
                })?,
                None => buf.clone()
            };
        }
    };
    Box::pin(stream)
}
