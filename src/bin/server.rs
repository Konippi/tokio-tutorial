use mini_redis::{
    Command::{self, Get, Set},
    Connection, Frame,
};
use tokio::net::{TcpListener, TcpStream};

use tokio_tutorial::db::Db;

#[tokio::main]
pub async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379")
        .await
        .expect("failed to bind to address");

    println!("Listening on port 6379...");

    let db = Db::new();

    loop {
        let (socket, _) = listener.accept().await.unwrap();

        println!("Accepted connection from {:?}", socket.peer_addr());

        let db = db.clone();

        tokio::spawn(async move {
            process(socket, db).await;
        });
    }
}

async fn process(socket: TcpStream, db: Db) {
    let mut connection = Connection::new(socket);

    while let Some(frame) = connection.read_frame().await.unwrap() {
        let response = match Command::from_frame(frame).unwrap() {
            Set(cmd) => {
                db.insert(cmd.key().to_string(), cmd.value().clone());
                Frame::Simple("OK".to_string())
            }
            Get(cmd) => {
                if let Some(value) = db.get(cmd.key()) {
                    Frame::Bulk(value.clone())
                } else {
                    Frame::Null
                }
            }
            cmd => panic!("unimplemented {:?}", cmd),
        };

        connection.write_frame(&response).await.unwrap();
    }
}
