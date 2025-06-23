mod jobs;
mod workers;
mod handlers;

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

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db_pool = MySqlPool::connect(&db_url).await.unwrap();
    let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");
    let redis_client = redis::Client::open(redis_url).unwrap();

    let (tx, rx) = mpsc::channel::<String>(100);
    let tx = Arc::new(tx);
    let rx = Arc::new(Mutex::new(rx));

    let mut workers = Vec::with_capacity(WORKER_COUNT);

    for i in 0..WORKER_COUNT {
        workers.push(ImageWorker::start(i));
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