# rust-image-upload-processor

Locally

- Creating DB automatically   
    CREATE DATABASE image_processor;  
    CREATE USER 'rustuser'@'localhost' IDENTIFIED BY 'rustpass';  
    GRANT ALL PRIVILEGES ON image_processor.* TO 'rustuser'@'localhost';  
    FLUSH PRIVILEGES;
- CREATE TABLE files (  
    id SERIAL PRIMARY KEY,  
    file_name VARCHAR(255) NOT NULL,  
    directory VARCHAR(255),  
    type VARCHAR(100),  
    original_name VARCHAR(255),  
    origin VARCHAR(100)  
    );
    
- GRANT ALL PRIVILEGES ON image_processor.* TO 'rustuser'@'localhost';  
    FLUSH PRIVILEGES;
    
- .env with DATABASE_URL=mysql://rustuser:rustpass@localhost/image_processor
- cargo install sqlx-cli --no-default-features --features mysql
- cargo install cargo-dotenv
- cargo dotenv sqlx prepare
- cargo run

Test request when project is running:
- create image inside the root directory
- url -X POST -F "image=@imagename;type=image/jpeg" http://localhost:3000/upload


Docker (not ready)

docker build -t rust-image-upload-processor .
docker run --rm rust-image-upload-processor