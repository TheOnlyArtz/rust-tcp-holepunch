use clap::Parser;
use std::collections::HashMap;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender};

#[derive(Parser, Debug)]
#[clap(name = "Tcp-PunchHole-Server", version, author, about = "A Tcp-PunchHole-Server")]
struct Cli {
    #[clap(long)]
    ip: Option<String>,
    #[clap(long)]
    port: Option<u16>,
    #[clap(long, takes_value = false)]
    enable_ipv6: bool
}

#[derive(Clone, Copy, Debug)]
enum IPType{
    V4,
    V6
}

#[derive(Debug, Clone)]
struct Peer {
    pub local_address: String,
    pub local_port: u16,
    pub remote_address: String,
    pub remote_port: u16,
}

fn handle_client(mut socket: TcpStream, peers: Arc<Mutex<Vec<Peer>>>, hosts_tx: Sender<(std::string::String, std::string::String)>) -> std::io::Result<()> {
    let stringified_address = socket.peer_addr().unwrap().ip().to_string();
    let socket_port = socket.peer_addr().unwrap().port();
    loop {
        let mut buf = [0; 1024];
        let size = socket.read(&mut buf);

        if buf.len() == 0 || size.is_err() {
            let mut lock = peers.lock().unwrap();
            let mut iter = lock.iter();

            let i = iter.position(|x| {
                x.remote_address == stringified_address && x.remote_port == socket_port
            });

            if let Some(index) = i {
                lock.remove(index);
                // println!("Removed: {:?}", lock);
            }

            drop(lock);
            return Ok(());
        }

        // since it's a POC - it needs to be set and done so let us assume
        // that the message looks like xxx.xxx.xxx.xxx:ppppp or x:x:x:x:x:x:x:x:ppppp
        let (local_address, local_port) = ip_parser(String::from_utf8(buf[..size.unwrap()].to_vec()).unwrap());

        println!("[INCOMING] from {} => [{}]:{}", socket.peer_addr().unwrap(), local_address, local_port);

        let peer = Peer {
            local_address,
            local_port: local_port.parse::<u16>().unwrap(),

            remote_address: socket.peer_addr().unwrap().ip().to_string(),
            remote_port: socket
                .peer_addr()
                .unwrap()
                .port()
                .to_string()
                .parse::<u16>()
                .unwrap(),
        };

        let mut lock = peers.lock().unwrap();
        lock.push(peer);

        for p in lock.iter() {
            let filtered =
                filter_peers(&lock, String::from(&p.remote_address), p.remote_port);

            if filtered.len() > 0 {
                let sent = hosts_tx.send((format!("{}:{}", p.remote_address, p.remote_port), encode_peers(&filtered)));
                if let Err(e) = sent {
                    println!("Error sending payload to channel {}", e);
                }
            }
        }

        drop(lock);
    }
}

fn ip_parser(buf: String) -> (String, String) {
    let (ip, port) = buf.split_at(buf.rfind(":").unwrap());
    (ip.to_owned(), port.split_at(1).1.to_owned())
}

fn main() -> std::io::Result<()> {

    let cli = Cli::parse();

    let port = cli.port.unwrap_or(3000.to_owned());
    let (ip_type, default_ip) = if cli.enable_ipv6 { (IPType::V6, "0:0:0:0:0:0:0:1") } else { (IPType::V4, "127.0.0.1") };
    let ip = cli.ip.unwrap_or(default_ip.to_owned());
    println!("[CONFIG] IP Type: {:?}, Addr: {}:{}", ip_type, ip, port);

    let listener = TcpListener::bind(format!("{}:{}", ip, port))?;
    let peers: Arc<Mutex<Vec<Peer>>> = Arc::new(Mutex::new(Vec::<Peer>::new()));
    let connections: Arc<Mutex<HashMap<String, TcpStream>>> = Arc::new(Mutex::new(HashMap::<String, TcpStream>::new()));
    let (hosts_tx, hosts_rx) = channel::<(String, String)>();

    let cloned_connections = Arc::clone(&connections);

    // This is the loop which is listening for incoming messages 
    // from the channel
    // the idea behind this channel is to send payloads to the desired
    // socket connections
    std::thread::spawn(move || {
        loop {
            let recv = hosts_rx.recv();

            if recv.is_err() {
                println!("Recv error !");
                break
            }

            // Get the desired socket via the key
            let (key, payload) = recv.unwrap();
            let mut lock = cloned_connections.lock().unwrap();
            let target = lock.get_mut(&key);

            if let Some(target) = target {
                let written = target.write(payload.as_bytes());
                if let Err(e) = written {
                    println!("Error sending payload to {}: {}", key, e);
                }
            }
            drop(lock);
        }    
    });

    for stream in listener.incoming() {
        let peers_arc = Arc::clone(&peers);
        let stream = stream.unwrap();

        let stringified_address = stream.peer_addr().unwrap().ip().to_string();
        let stream_port = stream.peer_addr().unwrap().port();

        let key = format!("{}:{}", stringified_address, stream_port);

        let mut lock = connections.lock().unwrap();
        lock.insert(key, stream.try_clone().unwrap());
        drop(lock);

        let hosts_tx_clone = hosts_tx.clone();
        std::thread::spawn(move || {
            let handled = handle_client(stream, peers_arc, hosts_tx_clone);
            if let Err(e) = handled {
                println!("Crashed ! {}", e);
            }
        });
    }

    Ok(())
}

// filter is remote address
fn filter_peers(peers: &Vec<Peer>, filter_ip: String, filter_port: u16) -> Vec<Peer> {
    let mut result: Vec<Peer> = vec![];

    for i in peers {
        if i.remote_address == filter_ip && i.remote_port == filter_port {
            continue;
        }

        result.push(i.clone());
    }

    result
}

fn encode_peers(peers: &Vec<Peer>) -> String {
    let mut keys: Vec<String> = vec![];
    // let result = String::from("");

    for p in peers {
        keys.push(format!("{}:{}|{}:{}", p.remote_address, p.remote_port, p.local_address, p.local_port));
    }

    keys.join(",")
}