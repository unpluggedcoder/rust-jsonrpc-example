#[macro_use]
extern crate log;

#[macro_use]
extern crate lazy_static;

use futures::future::{FutureExt, TryFutureExt};

use std::collections::HashMap;
use std::env;
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use jsonrpc_core::{self, types, BoxFuture, MetaIoHandler, Metadata, Result};
use jsonrpc_derive::rpc;
use jsonrpc_http_server::{hyper, ServerBuilder};
use tokio::runtime;
use tokio::sync::mpsc::{self, Sender, UnboundedSender};

lazy_static! {
    static ref REQ_ID: AtomicUsize = AtomicUsize::new(0);
}

fn async_wait<T: Send + 'static>(
    fut: impl Future<Output = Result<T>> + Send + 'static,
) -> BoxFuture<T> {
    let compat_fut = fut.boxed().compat();
    Box::new(compat_fut)
}

#[derive(Default, Clone)]
pub struct Meta(pub Option<UnboundedSender<(usize, Sender<usize>)>>);
impl Metadata for Meta {}

#[rpc(server)]
pub trait Rpc {
    type Metadata;
    /// Performs asynchronous operation
    #[rpc(meta, name = "beFancy")]
    fn call(&self, meta: Self::Metadata) -> BoxFuture<usize>;
}

#[derive(Default, Clone)]
struct RpcImpl;

impl Rpc for RpcImpl {
    type Metadata = Meta;

    fn call(&self, meta: Self::Metadata) -> BoxFuture<usize> {
        let id = REQ_ID.fetch_add(1, Ordering::SeqCst);
        let (tx, mut rx) = mpsc::channel::<usize>(1);
        if let Some(sender) = meta.0 {
            sender.send((id, tx)).unwrap();
        }

        let resp_fut = async move {
            match rx.recv().await {  // Block at here
                Some(id) => Ok(id),
                None => Err(types::Error::new(types::ErrorCode::InternalError)),
            }
        };

        async_wait(resp_fut)
    }
}

fn main() {
    env::set_var("RUST_LOG", "info");
    pretty_env_logger::init();

    let mut rt = runtime::Builder::new()
        .threaded_scheduler()
        .enable_all()
        .build()
        .expect("Runtime build failed.");

    let (broker_sender, mut broker_receiver) = mpsc::unbounded_channel();

    rt.spawn(async {
        let mut io = MetaIoHandler::default();
        let rpc = RpcImpl;

        io.extend_with(rpc.to_delegate());

        let _server = ServerBuilder::new(io)
            .meta_extractor(move |_: &hyper::Request<hyper::Body>| {
                info!("Meta extractor called.");
                Meta(Some(broker_sender.clone()))
            })
            .start_http(&"127.0.0.1:9527".parse().unwrap())
            .expect("Unable to start RPC server");

        _server.wait();
    });

    rt.block_on(async move {
        let mut rpc_resps: HashMap<usize, Sender<usize>> = HashMap::new();
        info!("Borker loop start...");
        loop {
            if let Some((id, mut sender)) = broker_receiver.recv().await {
                info!("Broker received: id({}).", id);
                // Sleep for awhile
                thread::sleep(Duration::from_secs(1));

                sender.send(id * id).await.unwrap();
                info!("Broker sent: id({})", id);
                rpc_resps.insert(id, sender);
            } else {
                info!("Broker channel broken.");
                break;
            }
        }
        info!("Broker loop finished.");
    });
}
