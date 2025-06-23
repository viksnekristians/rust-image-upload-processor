use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize, Debug)]
pub struct ImagePostUploadJob {
    pub id: usize,
    pub file_name: String,
    pub dir: String,
}

impl ImagePostUploadJob {
    pub fn generate_thumbnail(&self) {
        let mut file_path = std::env::current_dir().unwrap();
        file_path.push(&self.dir);
        let mut thumb_path = file_path.clone();
        thumb_path.push("thumbnails");
        file_path.push(&self.file_name);
        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("jpg");

        let thumb_name = format!(
            "{}_thumb.{}",
            std::path::Path::new(&self.file_name).file_stem().and_then(|s| s.to_str()).unwrap_or("thumb"),
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