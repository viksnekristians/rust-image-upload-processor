mod jobs;
mod workers;
mod handlers;
mod logger;

use jobs::ImagePostUploadJob;
use workers::ImageWorker;
use workers::Worker;
use handlers::{handler, upload_image};
use axum::{
    routing::{get, post},
    Router,
    extract::Multipart,
};
use std::{sync::{Arc, Mutex}, thread};
use tokio::sync::mpsc;
use tokio::net::TcpListener;
use sqlx::mysql::MySqlPool;
use axum::extract::State;

const WORKER_COUNT: usize = 4;
const LOG_CHANNEL_SIZE: usize = 100;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db_pool = MySqlPool::connect(&db_url).await.unwrap();
    let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");
    let redis_client = redis::Client::open(redis_url).unwrap();

    let (log_sender, log_receiver) = mpsc::channel::<String>(LOG_CHANNEL_SIZE);
    let log_sender = Arc::new(log_sender);
    let log_receiver = Arc::new(Mutex::new(log_receiver));

    logger::start_logger(log_receiver.clone());
    let mut workers = Vec::with_capacity(WORKER_COUNT);

    for i in 0..WORKER_COUNT {
        workers.push(ImageWorker::start(i, log_sender.clone()));
    }

    let app = Router::new()
        .route("/", get(handler))
        .route("/upload", post(
            {
                move |State(db_pool): State<MySqlPool>, multipart: Multipart| upload_image(State(db_pool), redis_client.clone(), multipart)
            }
        )).with_state(db_pool.clone());

    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("Listening on http://{}", listener.local_addr().unwrap());

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}