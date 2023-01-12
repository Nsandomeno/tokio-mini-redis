use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let (tx1, mut rx1) = mpsc::channel(128);
    let (tx2, mut rx2) = mpsc::channel(128);
    let (tx3, mut rx3) = mpsc::channel(128);

    loop {
        // selects over three diff channel receivers
        let msg = tokio::select! {
            //If when select! is evaluated, multiple channels have pending messages, 
            //only one channel has a value popped. 
            //All other channels remain untouched,
            // and their messages stay in those channels until the next loop iteration.
            // No messages are lost.
            Some(msg) = rx1.recv() => msg,
            Some(msg) = rx2.recv() => msg,
            Some(msg) = rx3.recv() => msg,
            else => { break }
        };

        println!("Got {:?}", msg);
    }

    println!("All channels have been closed.");
}