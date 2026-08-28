use crate::domain::market::SocketServer;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

impl SocketServer {
    pub fn new() -> Self {
        let (tx, _) = tokio::sync::broadcast::channel(100);
        Self { tx }
    }

    pub fn broadcast(&self, msg: &str) {
        let _ = self.tx.send(msg.to_string());
    }

    pub async fn run(&self, addr: &str) -> std::io::Result<()> {
        let listener = TcpListener::bind(addr).await?;
        println!("WebSocket server running on ws://{addr}");

        loop {
            let Ok((stream, peer)) = listener.accept().await else {
                continue;
            };
            println!("WebSocket connection from {peer}");

            let mut rx = self.tx.subscribe();

            tokio::spawn(async move {
                let ws_stream = match tokio_tungstenite::accept_async(stream).await {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("WebSocket handshake failed for {peer}: {e}");
                        return;
                    }
                };

                let (mut ws_sender, mut ws_receiver) = ws_stream.split();

                // forward broadcast messages to this client
                let send_task = tokio::spawn(async move {
                    while let Ok(msg) = rx.recv().await {
                        if ws_sender.send(Message::Text(msg.into())).await.is_err() {
                            break;
                        }
                    }
                });

                // drain incoming messages (client pings / close frames)
                let recv_task = tokio::spawn(async move {
                    while let Some(Ok(_)) = ws_receiver.next().await {}
                });

                tokio::select! {
                    _ = send_task => {}
                    _ = recv_task => {}
                }
            });
        }
    }
}
