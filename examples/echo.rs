//! Echo example.
//! Use `nc 127.0.0.1 3456` to connect.


use mini_rust_runtime_io_uring::executor::Executor;
use mini_rust_runtime_io_uring::tcp::TcpListener;

fn main() {
    let ex = Executor::new();
    ex.block_on(serve);
}

async fn serve() {
    let addr = "0.0.0.0:3456";
    let listener = TcpListener::bind(addr).unwrap();
    println!("listen on {addr}");
    loop {
        let (stream, addr) = listener.accept().await.unwrap();
        println!("accept a new connection from {} successfully", addr);
        let f = async move {
            let mut buf = [0; 4096];
            loop {
                match stream.read(&mut buf).await {
                    Ok(n) => {
                        if n == 0 || stream.write(&buf[..n]).await.is_err() {
                            return;
                        }
                    }
                    Err(_) => {
                        return;
                    }
                }
            }
        };
        Executor::spawn(f);
    }
}
