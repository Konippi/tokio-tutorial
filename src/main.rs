use std::collections::HashMap;

use mini_redis::{
    Command::{self, Get, Set},
    Connection, Frame,
};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
pub async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379")
        .await
        .expect("failed to bind to address");

    loop {
        let (scoket, _) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            process(scoket).await;
        });
    }
}

async fn process(socket: TcpStream) {
    let mut db = HashMap::<String, Vec<u8>>::new();
    let mut connection = Connection::new(socket);

    while let Some(frame) = connection.read_frame().await.unwrap() {
        let response = match Command::from_frame(frame).unwrap() {
            Set(cmd) => {
                db.insert(cmd.key().to_string(), cmd.value().to_vec());
                Frame::Simple("OK".to_string())
            }
            Get(cmd) => {
                if let Some(value) = db.get(cmd.key()) {
                    Frame::Bulk(value.clone().into())
                } else {
                    Frame::Null
                }
            }
            cmd => panic!("unimplemented {:?}", cmd),
        };

        connection.write_frame(&response).await.unwrap();
    }
}
