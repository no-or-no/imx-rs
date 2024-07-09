use std::io;
use std::time::Duration;
use log::error;
use tokio::time::sleep;

use imx::Client;

mod server;

const SERVER: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() {
    logger::init();

    server::run(SERVER);

    sleep(Duration::from_secs(1)).await;

    let client = match Client::new(SERVER) {
        Ok(c) => c,
        Err(e) => {
            error!("{}", e);
            return;
        }
    };
    client.start();

    // std::thread::sleep(Duration::from_secs(2));
    // client.pause();
    //
    // std::thread::sleep(Duration::from_secs(3));
    // client.resume();
    //
    // std::thread::sleep(Duration::from_secs(2));
    // client.stop();

    let mut input = String::new();
    loop {
        input.clear();
        match io::stdin().read_line(&mut input) {
            Ok(_n) => {
                let line = input.trim();
                match line {
                    "/start" => client.start(),
                    "/stop" => client.stop(),
                    "/exit" => {
                        client.release();
                        std::process::exit(0)
                    }
                    _ => {
                        if line.starts_with("/send ") {
                            // let msg = line.replace("/send ", "");
                            // let r = client.send(&msg).await;
                            // println!("result: {r:?}");
                        } else {
                            error!("input: {input}")
                        }
                    }
                }
            }
            Err(error) => println!("error: {error}"),
        }
    }
}

mod logger {
    use colored::Colorize;
    use log::{Level, LevelFilter, Metadata, Record};

    pub fn init() {
        log::set_logger(&SimpleLogger)
            .map(|()| log::set_max_level(LevelFilter::Debug))
            .unwrap_or_else(|err| println!("{}", err));
    }

    struct SimpleLogger;

    impl log::Log for SimpleLogger {
        fn enabled(&self, metadata: &Metadata) -> bool {
            metadata.level() <= Level::Debug
        }

        fn log(&self, record: &Record) {
            if self.enabled(record.metadata()) {
                if record.target() == "server" {
                    println!("{}", format!("[Server] {}", record.args()).cyan());
                } else {
                    match record.level() {
                        Level::Error => {
                            println!("{}", format!("[Client] {}", record.args()).red());
                        }
                        _ => {
                            println!("{}", format!("[Client] {}", record.args()).purple());
                        }
                    }
                }
            }
        }

        fn flush(&self) {}
    }
}
