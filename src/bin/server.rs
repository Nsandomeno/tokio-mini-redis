use bytes::Bytes;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};
use tokio::net::{TcpStream, TcpListener};
use mini_redis::{Connection, Frame, client};

type Db = Arc<Mutex<HashMap<String, Bytes>>>;
type ShardedDb = Arc<Vec<Mutex<HashMap<String, Vec<u8>>>>>;
type Responder<T> = oneshot::Sender<mini_redis::Result<T>>;

#[derive(Debug)]
enum Command {
    Get {
        key: String,
        resp: Responder<Option<Bytes>>,
    },
    Set {
        key: String,
        val: Bytes,
        resp: Responder<Option<Bytes>>,
    }
}

fn new_sharded_db(num_shards: usize) -> ShardedDb {
    let mut db = Vec::with_capacity(num_shards);
    for _ in 0..num_shards {
        db.push(Mutex::new(HashMap::new()));
    }
    Arc::new(db)
}

async fn process(socket: TcpStream, db: Db) {
    use mini_redis::Command::{self, Get, Set};
    // The Connection from mini-redis handles parsing frames from
    // the socket
    let mut connection = Connection::new(socket);

    while let Some(frame) = connection.read_frame().await.unwrap() {
        let response = match Command::from_frame(frame).unwrap() {
            Set(cmd) => {
                let mut db = db.lock().unwrap();
                db.insert(cmd.key().to_string(), cmd.value().clone());
                Frame::Simple("OK".to_string())
            }
            Get(cmd) => {
                let db = db.lock().unwrap();
                if let Some(value) = db.get(cmd.key()) {
                    // Frame::Bulk expects data to be of type Bytes. This
                    // type will be covered later. For now &Vec<u8> is converted to Bytes
                    // using into()
                    Frame::Bulk(value.clone())
                } else {
                    Frame::Null
                }
            }
            cmd => panic!("unimplemented {:?}", cmd),
        };

        // Writes the response to the client
        connection.write_frame(&response).await.unwrap();
    }
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();
    println!("Listening");

    let db: Db = Arc::new(Mutex::new(HashMap::new()));

    loop {
        // the second item contains the IP and port of the new connection.
        let (socket, _) = listener.accept().await.unwrap();
        // Clone the DB
        let db = db.clone();
        // A new task is spawned for each inbound socket. The socket is 
        // moved to the new task and processed there.

        println!("Accepted");
        tokio::spawn(async move {
            process(socket, db).await;
        });
    }
}
