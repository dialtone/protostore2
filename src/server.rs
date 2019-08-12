use tokio::codec::Framed;
use tokio::net::TcpStream;
use tokio::prelude::*;

use log::{debug, error, info, trace};

use crate::protocol::{Protocol, RequestType, Response};
use bytes::Bytes;

pub async fn handle_client(socket: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = Framed::new(socket, Protocol::new());
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
