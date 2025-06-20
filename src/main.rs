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
use rand::{distributions::Alphanumeric, Rng};
use image::ImageFormat;
use sqlx::mysql::MySqlPool;
use axum::extract::State;

async fn handler() -> StatusCode {
    println!("Received request");
    println!("Finished request");
    StatusCode::OK
}

async fn upload_image(State(db_pool): State<MySqlPool>, mut multipart: Multipart, tx: Arc<mpsc::Sender<ImagePostUploadJob>>) -> StatusCode {
    while let Some(field) = multipart.next_field().await.unwrap() {
        println!("Received field: {:?}", field);
        let field_name = field.name().unwrap_or("unnamed").to_string();
        let original_file_name = field.file_name().map(|name| name.to_string()).unwrap_or("unknown".to_string());
        let extension = field.file_name()
            .and_then(|name| std::path::Path::new(name).extension())
            .and_then(|ext| ext.to_str())
            .unwrap_or("bin");
        let allowed_extensions = ["jpg", "jpeg", "png", "gif", "bmp", "webp"];
        let is_valid_image_ext = allowed_extensions.iter().any(|&ext| ext.eq_ignore_ascii_case(extension));
        let content_type = field.content_type().unwrap_or("application/octet-stream").to_string();

        if content_type.starts_with("image/") && is_valid_image_ext {
            let file_name = format!(
                "{}.{}",
                rand::thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(16)
                    .map(char::from)
                    .collect::<String>(),
                extension
            );
            println!("Processing image: {} ({})", file_name, content_type);
            let data = field.bytes().await.unwrap();
            let format = image::guess_format(&data);
            match format {
                Ok(fmt) => {
                    // Try to decode the image to ensure it's valid
                    if image::load_from_memory_with_format(&data, fmt).is_ok() {
                        println!("Valid image of format: {:?}", fmt);
                        // You can now process or save the image
                    } else {
                        println!("Invalid image data");
                        continue;
                    }
                }
                Err(_) => {
                    println!("Unknown or unsupported image format");
                    continue;
                }
            } 
            let dir = "uploads";
            let mut path = std::env::current_dir().unwrap();
            path.push(dir);
            path.push(&file_name);
            let mut file = File::create(&path).await.unwrap();
            file.write_all(&data).await.unwrap();

            let rec = sqlx::query!(
                r#"
                INSERT INTO files (file_name, directory, type, original_name, origin)
                VALUES (?, ?, ?, ?, ?)
                "#,
                file_name,
                dir,
                "image",
                original_file_name,
                "web"
            )
            .execute(&db_pool)
            .await
            .unwrap();

            // Get the inserted id (for MySQL)
            let id = rec.last_insert_id();

            let row = sqlx::query!(
                r#"
                SELECT id, file_name, directory, type, original_name, origin
                FROM files
                WHERE id = ?
                "#,
                id as i64 // Make sure the type matches your DB schema
            )
            .fetch_one(&db_pool)
            .await
            .unwrap();

            println!(
                "Inserted file: id={:?}, file_name={:?}, directory={:?}, type={:?}, original_name={:?}, origin={:?}",
                row.id, row.file_name, row.directory, row.r#type, row.original_name, row.origin
            );

            let job = ImagePostUploadJob {
                id: row.id as usize,
                file_name: file_name.clone(),
                dir: dir.to_string(),
            };
            tx.send(job).await.unwrap();
        } else {
            println!("Ignoring non-image field: {}", field_name);
        }
    }

    // Return HTTP 200 OK to the client after processing all fields
    StatusCode::OK
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn start(id: usize, receiver: Arc<Mutex<mpsc::Receiver<ImagePostUploadJob>>>) -> Self {
        let thread = thread::spawn(move || {
            println!("[Worker {}] started", id);
            loop {
                let job = {
                    let mut rx = receiver.lock().unwrap();
                    rx.blocking_recv()
                };
                if let Some(job) = job {
                    println!("[Worker {}] Processing job: {} {}", id, job.dir, job.file_name);
                    job.generate_thumbnail();
                    println!("[Worker {}] Finished processing job: {}", id, job.file_name);
                } else {
                    println!("[Worker {}] No more jobs, exiting", id);
                    break;
                }
            }
        });
        Worker {id, thread: Some(thread)}
    }
}

struct ImagePostUploadJob {
    id: usize,
    file_name: String,
    dir: String,
}

impl ImagePostUploadJob {
    fn generate_thumbnail(&self) {
        let mut file_path = std::env::current_dir().unwrap();
        file_path.push(&self.dir);
        let mut thumb_path = file_path.clone();
        thumb_path.push("thumbnails");
        file_path.push(&self.file_name);
        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("jpg");

        let thumb_name = format!(
            "{}_thumb.{}",
            &self.file_name,
            ext
        );
        thumb_path.push(thumb_name);

        // Use blocking code for image processing
        match image::open(&file_path) {
            Ok(img) => {
                let thumb = img.thumbnail(200, 200); // classical thumbnail size
                if let Err(e) = thumb.save(&thumb_path) {
                    eprintln!("Failed to save thumbnail: {e}");
                } else {
                    println!("Thumbnail saved at: {}", thumb_path.display());
                }
            }
            Err(e) => {
                eprintln!("Failed to open image for thumbnail: {e}");
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let db_url = "mysql://rustuser:rustpass@localhost/image_processor";
    let db_pool = MySqlPool::connect(db_url).await.unwrap();
    let (tx, rx) = mpsc::channel::<ImagePostUploadJob>(100);
    let tx = Arc::new(tx);
    let rx = Arc::new(Mutex::new(rx));

    // Start N workers - constant?
    let worker_count = 4;
    let mut workers = Vec::with_capacity(worker_count);

    for i in 0..worker_count {
        let rx = rx.clone();
        workers.push(Worker::start(i, rx));
    }

    let app = Router::new()
        .route("/", get(handler))
        .route("/upload", post(
            {
                let tx = tx.clone();
                move |State(db_pool): State<MySqlPool>, multipart: Multipart| upload_image(State(db_pool), multipart, tx.clone())
            }
        )).with_state(db_pool.clone());

    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("Listening on http://{}", listener.local_addr().unwrap());

    // could be worth understanding what happens under the hood more (async request handling)
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}