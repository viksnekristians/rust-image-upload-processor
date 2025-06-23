use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

pub fn start_logger(rx: Arc<Mutex<mpsc::Receiver<String>>>) {
    std::thread::spawn(move || {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("log.txt")
            .expect("Failed to open log file");

        loop {
            let msg = {
                let mut rx = rx.lock().unwrap();
                rx.blocking_recv()
            };

            match msg {
                Some(line) => {
                    if let Err(e) = writeln!(file, "{}", line) {
                        eprintln!("Failed to write to log file: {}", e);
                    }
                }
                None => {
                    break;
                }
            }
        }
    });
}