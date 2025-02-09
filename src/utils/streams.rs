use std::pin::Pin;

use futures_util::Stream;
use orion::aead::streaming;
use tokio::io::AsyncReadExt;

/// DO NOT PROVIDE BAD `read_size`.
pub fn read_into_stream<'a, R>(
    reader: R,
    read_chunk_size: u32,
    mut sealer: Option<streaming::StreamSealer>,
) -> Pin<Box<impl Stream<Item = anyhow::Result<Vec<u8>>> + Send + 'static>>
where
    R: AsyncReadExt + Send + Unpin + 'static,
{
    let stream = async_stream::try_stream! {
        let mut reader = reader;

        loop {
            let mut buf: Vec<u8> = vec![0; read_chunk_size as usize];
            let mut is_last_chunk = false;

            let b_read = reader.read(&mut buf).await?;
            if b_read == 0 {
                break;
            } else if b_read < buf.len() {
                is_last_chunk = true;
                buf.truncate(b_read);
            }

            let buf = match &mut sealer {
                Some(s) => s.seal_chunk(&buf, match is_last_chunk {
                    true => &streaming::StreamTag::Finish,
                    false => &streaming::StreamTag::Message
                })?,
                None => buf
            };
            yield buf;
        }
    };
    Box::pin(stream)
}

//{
//    let stream = async_stream::try_stream! {
//        let read_size = match sealer {
//            Some(_) => read_size + (read_size / read_chunk_size as u64) * streaming::ABYTES as u64,
//            None => read_size
//        };
//        let reader = reader;
//
//        let mut b_read = 0;
//        let mut c = 0;
//        loop {
//            let mut buf: Vec<u8> = vec![0; read_chunk_size as usize];
//
//            println!("INSIDE THE STREAM");
//
//            // NOTE: assuming if n == 0, EOF is reached
//            println!("READING (read_chunk_size: {read_chunk_size})");
//            let n = reader.read(&mut buf).await?;
//            println!("READING...DONE, bytes: {n}");
//            if n == 0 {
//                break;
//            }
//
//            let mut _last = false;
//            if b_read + n as u64 >= read_size {
//                println!("{b_read} + {} >= {read_size}", buf.len());
//                buf.truncate((read_size - b_read).try_into().unwrap());
//                _last = true;
//            }
//
//            let buf = match &mut sealer {
//                Some(sealer) => {
//                    println!("sealing buf of len: {}", buf.len());
//                    c += 1;
//                    sealer.seal_chunk(&buf, match _last { true => &streaming::StreamTag::Finish, false => &streaming::StreamTag::Message })?
//                },
//                None => buf
//            };
//            println!("sealed buf? len: {}, _last: {_last}", buf.len());
//
//            b_read += buf.len() as u64;
//            println!("buf yield len: {} (n: {n}), total yield until now: {b_read}", buf.len());
//            yield buf;
//            if _last {
//                break;
//            }
//
//        }
//
//        println!("CHUNK COUNTER: {c}");
//        println!("READ COUNTER : {b_read}");
//    };
//
//    Box::pin(stream)
//}
