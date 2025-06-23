use serde::{Serialize, Deserialize};
use serde_json;
use redis::Commands;
use std::thread;
use tokio::sync::mpsc;
use std::sync::Arc;
use crate::ImagePostUploadJob;

pub trait Worker {
    fn start(id: usize, log_sender: Arc<mpsc::Sender<String>>) -> Self;
    fn join(self);
}

pub struct ImageWorker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>
}

impl Worker for ImageWorker {
    fn start(id: usize, log_sender: Arc<mpsc::Sender<String>>) -> Self {
        let thread = thread::spawn(move || {
            println!("[Worker {}] started", id);
            let redis_client = redis::Client::open(
                std::env::var("REDIS_URL").expect("REDIS_URL must be set")
            ).unwrap();
            let mut conn = redis_client.get_connection().unwrap();
            loop {
                let result: Option<(String, String)> = conn.blpop("image_jobs", 0.0).ok();
                if let Some((_queue, job_json)) = result {
                    let job: ImagePostUploadJob = serde_json::from_str(&job_json).unwrap();
                    println!("[Worker {}] Processing job for: {}", id, job.file_name);
                    job.generate_thumbnail();
                    log_sender.blocking_send(format!("Worker {} processed job for: {}", id, job.file_name)).unwrap();
                    println!("[Worker {}] Finished processing job for: {}", id, job.file_name);
                } else {
                    println!("[Worker {}] No more jobs, exiting", id);
                    break;
                }
            }
        });
        ImageWorker {id, thread: Some(thread)}
    }

    fn join(self) {
        if let Some(handle) = self.thread {
            handle.join().unwrap();
        }
    }
}