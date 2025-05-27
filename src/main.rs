use axum::{
    routing::{get, post},
    Router,

    http::StatusCode,
};
use std::{sync::{Arc, Mutex}, thread};
use tokio::sync::mpsc;
use tokio::net::TcpListener;

async fn handler(tx: Arc<mpsc::Sender<String>>) -> StatusCode {
    println!("Received request");
    // Simulate some async work
    tx.send("Hello".to_string()).await.unwrap();
    println!("Finished request");
    StatusCode::OK
}

fn start_worker(id: usize, rx: Arc<Mutex<mpsc::Receiver<String>>>) {
    thread::spawn(move || {
        println!("[Creating worker {}]", id);
        loop {
            let message = {
                let mut rx = rx.lock().unwrap();
                rx.blocking_recv()
            };

            match message {
                Some(msg) => {
                    println!("[Worker {}] Received: {}", id, msg);
                    // Simulate some work
                    thread::sleep(std::time::Duration::from_secs(3));
                    println!("[Worker {}] Finished processing: {}", id, msg);
                }
                None => {
                    println!("[Worker {}] Channel closed", id);
                    break;
                }
            }
        }
    });
}

#[tokio::main]
async fn main() {
    // mpsc - multiple producer, single consumer
    let (tx, rx) = mpsc::channel::<String>(100);
    // Arc is needed because Axum and workers are multi-threaded
    let tx = Arc::new(tx);
    let rx = Arc::new(Mutex::new(rx));

    // Start N workers
    let worker_count = 4;
    for i in 0..worker_count {
        let rx = rx.clone();
        start_worker(i, rx);
    }

    // Build router with routes
    let app = Router::new()
        .route("/", get(move || handler(tx.clone())));

    // Bind to address
    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("Listening on http://{}", listener.local_addr().unwrap());

    // could be worth understanding what happens under the hood more (async request handling)
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}