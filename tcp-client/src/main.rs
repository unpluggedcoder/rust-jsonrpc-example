#[macro_use]
extern crate log;

use std::env;

use async_std::io::{stdin, BufReader};
use async_std::net::{TcpStream, ToSocketAddrs};
use async_std::prelude::*;
use async_std::task;
use futures::select;
use futures::FutureExt;
use structopt::StructOpt;

/// Server start command options
#[derive(Debug, StructOpt)]
pub struct ClientOpt {
    /// The network interface on which to start the TCP server
    #[structopt(short = "i", long = "interface", default_value = "127.0.0.1")]
    pub interface: String,
    /// The port on which to start the TCP server
    #[structopt(short = "p", long = "port", default_value = "9527")]
    pub port: u16,
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }

    pretty_env_logger::init();

    let opt = ClientOpt::from_args();
    task::block_on(try_main(format!("{}:{}", opt.interface, opt.port))).unwrap();
}

async fn try_main(addr: impl ToSocketAddrs) -> Result<()> {
    let stream = TcpStream::connect(addr).await?;
    let (reader, mut writer) = (&stream, &stream);
    let reader = BufReader::new(reader);
    let mut lines_from_server = futures::StreamExt::fuse(reader.lines());

    let stdin = BufReader::new(stdin());
    let mut lines_from_stdin = futures::StreamExt::fuse(stdin.lines());
    let mut lines = Vec::new();
    loop {
        select! {
            line = lines_from_server.next().fuse() => match line {
                Some(line) => {
                    let line = line?;
                    println!("{}", line);
                },
                None => break,
            },
            line = lines_from_stdin.next().fuse() => match line {
                Some(line) => {
                    let line = line?;
                    if line.trim().is_empty() {
                        writer.write_all(&lines).await?;
                        writer.write_all(b"\n").await?;
                        info!("Sent");
                        lines.clear();
                    } else {
                        lines.extend_from_slice(line.as_bytes());
                    }
                }
                None => break,
            }
        }
    }
    Ok(())
}

