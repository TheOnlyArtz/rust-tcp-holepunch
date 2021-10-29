// use std::net::{SocketAddr, TcpListener};
// use socket2::{Socket, Domain, Type};
use net2::TcpBuilder;
use std::io::prelude::*;
use std::sync::{Arc, Mutex};

fn main() -> std::io::Result<()> {
    let connection_builder = TcpBuilder::new_v4()?;
    connection_builder.reuse_address(true).unwrap();

    let mut stream = connection_builder.connect("178.128.32.250:3000")?;

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
            listen(listen_on).unwrap();
        });

        // PUBLIC
        let cloned_stream = stream.try_clone().unwrap();
        let buf_clone = buf.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(200));

            let ips: Vec<&str> = buf_clone.split("|").collect();
            let connect_to = ips.get(0).unwrap();
            let laddr = cloned_stream.local_addr().unwrap().to_string();

            connect(&laddr, connect_to, connection_established, "public").unwrap();
        });

        // PRIVATE
        let cloned_stream = stream.try_clone().unwrap();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(200));

            let ips: Vec<&str> = (&buf).split("|").collect();
            let connect_to = ips.get(1).unwrap();
            let laddr = cloned_stream.local_addr().unwrap().to_string();

            connect(&laddr, connect_to, connection_established_clone, "private").unwrap();
        });
    }

    Ok(())
}

fn connect(
    laddr: &str,
    ip: &str,
    connection_established: Arc<Mutex<bool>>,
    flag: &'static str,
) -> std::io::Result<()> {
    let connection_builder = TcpBuilder::new_v4()?;
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

fn listen(ip: String) -> std::io::Result<()> {
    let server_builder = TcpBuilder::new_v4()?;
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
