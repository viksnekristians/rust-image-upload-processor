use axum::{
    extract::{Multipart, State},
    http::StatusCode,
};
use sqlx::mysql::MySqlPool;
use redis::Client as RedisClient;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use rand::{distributions::Alphanumeric, Rng};
use serde_json;
use redis::Commands;
use crate::ImagePostUploadJob;

pub async fn handler() -> StatusCode {
    println!("Received request");
    println!("Finished request");
    StatusCode::OK
}

pub async fn upload_image(
    State(db_pool): State<MySqlPool>,
    redis_client: RedisClient,
    mut multipart: Multipart,
) -> StatusCode {
    while let Some(field) = multipart.next_field().await.unwrap() {
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
            println!("Processing image: {}", file_name);
            let data = field.bytes().await.unwrap();
            let format = image::guess_format(&data);
            match format {
                Ok(fmt) => {
                    if !image::load_from_memory_with_format(&data, fmt).is_ok() {
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

            let id = rec.last_insert_id();

            let row = sqlx::query!(
                r#"
                SELECT id, file_name, directory, type, original_name, origin
                FROM files
                WHERE id = ?
                "#,
                id as i64 
            )
            .fetch_one(&db_pool)
            .await
            .unwrap();

            println!("Inserted file: {:?}", row.file_name);

            let job = ImagePostUploadJob {
                id: row.id as usize,
                file_name: file_name.clone(),
                dir: dir.to_string(),
            };
            let job_json = serde_json::to_string(&job).unwrap();

            let mut conn = redis_client.get_connection().unwrap();
            conn.rpush::<_, _, ()>("image_jobs", job_json).unwrap();
        } else {
            println!("Ignoring non-image field: {}", field_name);
        }
    }

    StatusCode::OK
}