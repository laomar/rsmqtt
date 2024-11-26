use rsmqtt::{mqtt::*, packet, packet::*};
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::thread;
fn main() {
    let ls = TcpListener::bind("0.0.0.0:1883").unwrap();
    loop {
        let conn = match ls.accept() {
            Ok((s, _)) => s,
            Err(_) => continue,
        };
        thread::spawn(|| serve(conn));
    }
}

fn serve(mut conn: TcpStream) {
    let mut buf = [0; 128];
    loop {
        match conn.read(&mut buf) {
            Ok(n) => {
                if n == 0 {
                    break;
                }
                println!("{:X}", buf[0]);
            }
            Err(e) => println!("Error: {:?}", e),
        }
    }
}
