use axum::{
    routing::{get, post},
    Router,
    http::StatusCode,
    extract::Multipart,
};
use std::{sync::{Arc, Mutex}, thread};
use tokio::sync::mpsc;
use tokio::net::TcpListener;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

async fn handler(tx: Arc<mpsc::Sender<String>>) -> StatusCode {
    println!("Received request");
    // Simulate some async work
    tx.send("Hello".to_string()).await.unwrap();
    println!("Finished request");
    StatusCode::OK
}

async fn upload_image(mut multipart: Multipart, tx: Arc<mpsc::Sender<String>>) -> StatusCode {
    while let Some(field) = multipart.next_field().await.unwrap() {
        println!("Received field: {:?}", field);
        let field_name = field.name().unwrap_or("unnamed").to_string();
        let file_name = "unnamed.jpg".to_string();
        let content_type = field.content_type().unwrap_or("application/octet-stream").to_string();

        if content_type.starts_with("image/") {
            println!("Processing image: {} ({})", file_name, content_type);
            let data = field.bytes().await.unwrap();
            println!("{data:?} bytes received for field: {}", field_name);

            // Be cautious with file paths in a real application!
            let path = std::path::Path::new(&file_name);
            let mut file = File::create(&path).await.unwrap();
            file.write_all(&data).await.unwrap();
            tx.send("image processing done".to_string()).await.unwrap();
        } else {
            println!("Ignoring non-image field: {}", field_name);
        }
    }

    StatusCode::OK
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn start(id: usize, receiver: Arc<Mutex<mpsc::Receiver<String>>>) -> Self {
        let thread = thread::spawn(move || {
            println!("[Worker {}] started", id);
            loop {
                let message = {
                    let mut rx = receiver.lock().unwrap();
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
        Worker {id, thread: Some(thread)}
    }
}

#[tokio::main]
async fn main() {
    // mpsc - multiple producer, single consumer
    let (tx, rx) = mpsc::channel::<String>(100);
    // Arc is needed because Axum and workers are multi-threaded
    let tx = Arc::new(tx);
    let rx = Arc::new(Mutex::new(rx));

    // Start N workers - constant?
    let worker_count = 4;
    let mut workers = Vec::with_capacity(worker_count);

    for i in 0..worker_count {
        let rx = rx.clone();
        workers.push(Worker::start(i, rx));
    }

    // Build router with routes
    // build in a separate function?
    let app = Router::new()
        .route("/", get({
            let tx = tx.clone();
            move || handler(tx)
        }))
        .route("/upload", post(
            {
                let tx = tx.clone();
                move |multipart: Multipart| upload_image(multipart, tx)
            }
        ));

    // Bind to address
    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("Listening on http://{}", listener.local_addr().unwrap());

    // could be worth understanding what happens under the hood more (async request handling)
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}