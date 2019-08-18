use log::{error, trace};
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

    pub async fn handle_client(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        trace!("handle client from tid {:?}", unsafe {
            libc::pthread_self()
        });

        // In a loop, read data from the socket and write the data back.
        while let Some(request) = self.client.next().await {
            let response = match request {
                Ok(ref req) => match req.reqtype {
                    RequestType::Read => self.respond_read(req),
                    RequestType::Write => self.respond_write(req),
                },
                Err(e) => {
                    error!("failed to read from client; err = {:?}", e);
                    return Ok(());
                }
            };
            match self.client.send(response).await {
                Ok(_) => (),
                // Error sending disconnects
                Err(e) => {
                    error!("failed to send to client; err = {:?}", e);
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    fn respond_read(&self, req: &Request) -> Response {
        Response {
            id: 32,
            body: Bytes::from(&b"read"[..]),
        }
    }

    fn respond_write(&self, req: &Request) -> Response {
        Response {
            id: 32,
            body: Bytes::from(&b"write"[..]),
        }
    }
}
