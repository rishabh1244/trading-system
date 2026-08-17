// for communicating the values to the frontend server through a socket server

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct SocketServer {
    clients: Arc<Mutex<Vec<TcpStream>>>,
}

impl SocketServer {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn broadcast(&self, msg: &str) {
        if let Ok(mut clients) = self.clients.lock() {
            clients.retain(|mut c| c.write_all(msg.as_bytes()).is_ok());
        }
    }

    pub fn run(&self, addr: &str) -> std::io::Result<()> {
        let listener = TcpListener::bind(addr)?;
        println!("Socket server running on {addr}");

        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            println!("Connection established!");

            let clients = self.clients.clone();
            self.clients
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(stream.try_clone()?);

            thread::spawn(move || {
                let mut buffer = [0; 1024];
                let mut stream = stream;
                loop {
                    match stream.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(n) => {
                            if let Ok(mut clients) = clients.lock() {
                                for client in clients.iter_mut() {
                                    let _ = client.write_all(&buffer[..n]);
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
            });
        }
        Ok(())
    }
}
