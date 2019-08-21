use log::{error, trace};
use std::cmp;
use std::sync::Arc;

use tokio::codec::Framed;
use tokio::net::TcpStream;
use tokio::prelude::*;

use bytes::Bytes;

use crate::protocol::{Protocol, Request, RequestType, Response};
use crate::toc::TableOfContents;

pub struct ProtostoreServer {
    toc: Arc<TableOfContents>,
    client: Framed<TcpStream, Protocol>,
    max_value_len: usize,
    short_circuit_reads: bool,
}

impl ProtostoreServer {
    pub fn new(
        socket: TcpStream,
        toc: Arc<TableOfContents>,
        max_value_len: usize,
        short_circuit_reads: bool,
    ) -> Self {
        let client = Framed::new(socket, Protocol::new());
        ProtostoreServer {
            toc,
            client,
            max_value_len,
            short_circuit_reads,
        }
    }

    pub async fn handle_client(&mut self) -> Result<(), std::io::Error> {
        trace!("handle client from tid {:?}", unsafe {
            libc::pthread_self()
        });

        // In a loop, read data from the socket and write the data back.
        while let Some(request) = self.client.next().await {
            let response = match request {
                Ok(ref req) => match req.reqtype {
                    RequestType::Read => self.respond_read(req).await?,
                    RequestType::Write => self.respond_write(req).await?,
                },
                Err(e) => {
                    error!("failed to read from client; err = {:?}", e);
                    return Err(e);
                }
            };
            trace!("Responding {:?}", response);
            match self.client.send(response).await {
                Ok(_) => (),
                // Error sending disconnects
                Err(e) => {
                    error!("failed to send to client; err = {:?}", e);
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    async fn respond_read(&self, req: &Request) -> Result<Response, std::io::Error> {
        if self.short_circuit_reads {
            return Ok(Response {
                id: req.id,
                body: Bytes::from(vec![0, 1, 2, 3]),
            });
        }
        trace!("Searching for: {:?}", req);
        let offset_and_len = self.toc.offset_and_len(&req.uuid);
        trace!("Offset and len: {:?}", offset_and_len);
        if let Some((offset, len)) = offset_and_len {
            let aligned_offset = offset - (offset % 512);
            let pad_left = offset - aligned_offset;
            let padded = pad_left + len as u64;
            let aligned_len = cmp::max(512, padded + 512 - (padded as u64 % 512));
            Ok(Response {
                id: req.id,
                body: Bytes::from(&b"read"[..]),
            })
        } else {
            Ok(Response {
                id: req.id,
                body: Bytes::new(),
            })
        }
    }

    async fn respond_write(&self, req: &Request) -> Result<Response, std::io::Error> {
        Ok(Response {
            id: req.id,
            body: Bytes::from(&b"write"[..]),
        })
    }
}
