// use std::net::{SocketAddr, TcpListener};
// use socket2::{Socket, Domain, Type};
use clap::Parser;
use net2::TcpBuilder;
use std::io::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Parser, Debug)]
#[clap(name = "Tcp-PunchHole-Client", version, author, about = "A Tcp-PunchHole-Client")]
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

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    let port = cli.port.unwrap_or(3000.to_owned());
    let (ip_type, default_ip) = if cli.enable_ipv6 { (IPType::V6, "0:0:0:0:0:0:0:1") } else { (IPType::V4, "127.0.0.1") };
    let ip = cli.ip.unwrap_or(default_ip.to_owned());
    println!("[CONFIG] IP Type: {:?}, Addr: {}:{}", ip_type, ip, port);

    let connection_builder = tcp_builder(&ip_type)?;
    connection_builder.reuse_address(true).unwrap();

    let mut stream = connection_builder.connect(format!("{}:{}", ip, port))?;

    let formatted_msg = format!(
        "{}:{}",
        stream.local_addr()?.ip(),
        stream.local_addr()?.port()
    );

    println!("[ME -> S] publishing local endpoint {}", formatted_msg);
    stream.write(formatted_msg.as_bytes())?;

    loop {
        let mut buf = [0; 1024];
        let size = stream.read(&mut buf).unwrap();
        let buf = String::from_utf8(buf[..size].to_vec()).unwrap();

        println!("[S -> ME] {}", buf);

        if size == 0 {
            break;
        }

        let connection_established = Arc::new(Mutex::new(false));
        let connection_established_clone = Arc::clone(&connection_established);
        let cloned_stream = stream.try_clone().unwrap();

        // listen
        std::thread::spawn(move || {
            let listen_on = cloned_stream.local_addr().unwrap().to_string();
            println!(
                "[LISTENING] on the same port used to connect to S {}",
                listen_on
            );
            listen(listen_on, &ip_type).unwrap();
        });

        // PUBLIC
        let cloned_stream = stream.try_clone().unwrap();
        let buf_clone = buf.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(200));

            let ips: Vec<&str> = buf_clone.split("|").collect();
            let connect_to = ips.get(0).unwrap();
            let laddr = cloned_stream.local_addr().unwrap().to_string();

            connect(&laddr, connect_to, connection_established, "public", &ip_type).unwrap();
        });

        // PRIVATE
        let cloned_stream = stream.try_clone().unwrap();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(200));

            let ips: Vec<&str> = (&buf).split("|").collect();
            let connect_to = ips.get(1).unwrap();
            let laddr = cloned_stream.local_addr().unwrap().to_string();

            connect(&laddr, connect_to, connection_established_clone, "private", &ip_type).unwrap();
        });
    }

    Ok(())
}

fn connect(
    laddr: &str,
    ip: &str,
    connection_established: Arc<Mutex<bool>>,
    flag: &'static str,
    ip_type: &IPType
) -> std::io::Result<()> {
    let connection_builder = tcp_builder(ip_type)?;
    connection_builder.reuse_address(true).unwrap();
    connection_builder.bind(laddr).unwrap();

    loop {
        let established = *connection_established.lock().unwrap();

        if established {
            println!("Breaking {} loop cause the other one connected", flag);
            break;
        }

        drop(established);

        println!(
            "[ME -> B] Trying to connect to {} which is {} from {}",
            ip, flag, laddr
        );
        let stream = connection_builder.connect(ip);

        if stream.is_err() {
            println!("[ME -> B] Connection failed: repeating");
            continue;
        }

        println!("Connected to {} successfully!", ip);

        *connection_established.lock().unwrap() = true;
        let mut stream = stream.unwrap();

        loop {
            let mut buf = [0; 1024];
            let size = stream.read(&mut buf);

            if size.is_err() {
                continue;
            }

            let size = size.unwrap();
            let _buf = String::from_utf8(buf[..size].to_vec()).unwrap();

            if size == 0 {
                println!("Other peer closed connection!");
                break
            }
        }
    }
    Ok(())
}

fn listen(ip: String, ip_type :&IPType) -> std::io::Result<()> {
    let server_builder = tcp_builder(ip_type)?;
    println!("Listening b: {}", ip);
    server_builder
        .reuse_address(true)
        .unwrap()
        .bind(ip)
        .unwrap();

    let server = server_builder.listen(1)?;
    for stream in server.incoming() {
        let stream = stream.unwrap();

        println!(
            "[B -> ME] PEER: {:?} | LOCAL: {:?}",
            stream.peer_addr().unwrap(),
            stream.local_addr().unwrap()
        );
    }
    Ok(())
}

fn tcp_builder(ip_type: &IPType) -> std::io::Result<TcpBuilder> {
    match ip_type {
        IPType::V4 => {TcpBuilder::new_v4()}
        IPType::V6 => {TcpBuilder::new_v6()}
    }
}
