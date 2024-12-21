use std::str::from_utf8;

use mini_redis::{client, Result};

#[tokio::main]
pub async fn main() -> Result<()> {
    let mut client = client::connect("127.0.0.1:6379").await?;

    client.set("user", "password".into()).await?;

    let key = "user";
    let result = client.get(key).await?;
    match result {
        Some(value) => {
            println!("Got: {}", from_utf8(&value)?);
        }
        None => {
            println!("Not found: {}", key);
        }
    }

    Ok(())
}
