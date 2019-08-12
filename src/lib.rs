#![feature(async_await)]

mod protocol;
mod server;

pub use server::handle_client;
