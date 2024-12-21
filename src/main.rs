use mini_redis::{Connection, Frame};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
pub async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379")
        .await
        .expect("failed to bind to address");

    loop {
        let (scoket, _) = listener.accept().await.expect("failed to accept");
        tokio::spawn(async move {
            process(scoket).await;
        });
    }
}

async fn process(socket: TcpStream) {
    let mut connection = Connection::new(socket);

    if let Some(frame) = connection.read_frame().await.expect("failed to read frame") {
        println!("GOT: {:?}", frame);

        let response = Frame::Error("unimplemented".to_string());
        connection
            .write_frame(&response)
            .await
            .expect("failed to write frame");
    }
}
