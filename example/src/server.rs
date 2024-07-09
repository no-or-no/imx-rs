use log::{error, info};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub fn run(addr: &'static str) {
    std::thread::spawn(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                run_inner(addr).await.unwrap();
            });
    });
}

async fn run_inner(addr: &str) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;

    loop {
        let (mut socket, addr) = listener.accept().await?;

        info!(target: "server", "Accept: {:?}", addr);

        tokio::spawn(async move {
            let mut buf = [0; 1024];

            // In a loop, read data from the socket and write the data back.
            loop {
                let n = match socket.read(&mut buf).await {
                    // socket closed
                    Ok(n) if n == 0 => return,
                    Ok(n) => n,
                    Err(e) => {
                        error!(target: "server", "failed to read from socket; err = {:?}", e);
                        return;
                    }
                };

                let data = &buf[0..n];
                match std::str::from_utf8(&data[20..n]) {
                    Ok(text) => {
                        info!(target: "server", "Received data: \n{}", text);
                    }
                    Err(_) => {
                        info!(target: "server", "Server received data: \n{:?}", &data);
                    }
                }

                let res_str = "HTTP/1.1 200 OK\r\nContent-Length:12\r\nContent-Type:application/json; charset=\"utf-8\"\r\n\r\n{\"status\":0}";
                info!(target: "server", "Response: \n{}", &res_str);

                let res = res_str.as_bytes();

                // Write the data back
                if let Err(e) = socket.write_all(&res/*&buf[0..n]*/).await {
                    error!(target: "server", "failed to write to socket; err = {:?}", e);
                    return;
                }
            }
        });
    }
}