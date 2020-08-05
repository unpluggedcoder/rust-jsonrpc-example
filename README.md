# rust-jsonrpc-example
Json-RPC example in Rust by using https://github.com/paritytech/jsonrpc

This repo try to use `tokio = 0.2` with `jsonrpc-tcp-server = 14.2` which use `futures = 0.1`.

There is a TCP client beside the example crate to demonstrate the jsonrpc interaction.

I try to implement the async Rpc response with `async` --> `std::future::Future`. But it stuck at the `await` in the `async` block.

```rust
        let resp_fut = async move {
            match rx.recv().await {
                // Block at here
                Some(id) => Ok(id),
                None => Err(types::Error::new(types::ErrorCode::InternalError)),
            }
        };
```

I thought it same compatibility problem between `tokio = 0.2` and `futures = 0.1`.

BUT if I change the transport to `jsonrpc-http-sever`(feature in the `example/Cargo.toml`), it works fine.
