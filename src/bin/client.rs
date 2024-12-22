use mini_redis::client;
use tokio::sync::{mpsc, oneshot};

use bytes::Bytes;

#[derive(Debug)]
pub enum Cmd {
    Get {
        key: String,
        resp: Responder<Option<Bytes>>,
    },
    Set {
        key: String,
        value: Bytes,
        resp: Responder<()>,
    },
}

type Responder<T> = oneshot::Sender<mini_redis::Result<T>>;

#[tokio::main]
pub async fn main() {
    use Cmd::{Get, Set};

    let (tx, mut rx) = mpsc::channel::<Cmd>(32);
    let tx2 = tx.clone();

    let manager = tokio::spawn(async move {
        // Connect to the mini-redis server
        let mut client = client::connect("127.0.0.1:6379").await.unwrap();

        // Start receiving messages from other tasks
        while let Some(cmd) = rx.recv().await {
            match cmd {
                Get { key, resp } => {
                    let res = Ok(client.get(&key).await.unwrap());
                    let _ = resp.send(res);
                }
                Set { key, value, resp } => {
                    let res = Ok(client.set(&key, value).await.unwrap());
                    let _ = resp.send(res);
                }
            }
        }
    });

    let t1 = tokio::spawn(async move {
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Set {
            key: "foo".to_string(),
            value: "bar".into(),
            resp: resp_tx,
        };
        tx.send(cmd).await.unwrap();
        let resp = resp_rx.await.unwrap();
        println!("GOT: {:?}", resp);
    });

    let t2 = tokio::spawn(async move {
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Get {
            key: "foo".to_string(),
            resp: resp_tx,
        };
        tx2.send(cmd).await.unwrap();
        let resp = resp_rx.await.unwrap();
        println!("GOT: {:?}", resp);
    });

    t1.await.unwrap();
    t2.await.unwrap();
    manager.await.unwrap();
}
