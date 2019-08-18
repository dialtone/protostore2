use log::{error, trace};
use std::sync::Arc;

use tokio::codec::Framed;
use tokio::net::TcpStream;
use tokio::prelude::*;

use bytes::Bytes;

use crate::protocol::{Protocol, RequestType, Response};
use crate::toc::TableOfContents;

pub struct ProtostoreServer {
    toc: Arc<TableOfContents>,
    socket: TcpStream,
}

impl ProtostoreServer {
    pub fn new(socket: TcpStream, toc: Arc<TableOfContents>) -> Self {
        ProtostoreServer { toc, socket }
    }

    pub async fn handle_client(self) -> Result<(), Box<dyn std::error::Error>> {
        let mut client = Framed::new(self.socket, Protocol::new());
        trace!("handle client from tid {:?}", unsafe {
            libc::pthread_self()
        });

        // In a loop, read data from the socket and write the data back.
        while let Some(request) = client.next().await {
            let response = match request {
                Ok(ref req) => match req.reqtype {
                    RequestType::Read => Response {
                        id: 32,
                        body: Bytes::from(&b"read"[..]),
                    },
                    RequestType::Write => Response {
                        id: 32,
                        body: Bytes::from(&b"write"[..]),
                    },
                },
                Err(e) => {
                    error!("failed to read from client; err = {:?}", e);
                    return Ok(());
                }
            };
            match client.send(response).await {
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
}
