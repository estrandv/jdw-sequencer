extern crate rosc;

use std::net::{SocketAddrV4, UdpSocket};
use std::str::FromStr;

use rosc::{OscPacket};
use rosc::encoder;
use crate::config;

pub struct OSCClient {
    socket: UdpSocket,
    out_addr: SocketAddrV4,
}

impl OSCClient {
    pub fn new() -> OSCClient {
        let cfg = config::Config::get();
        let addr = match SocketAddrV4::from_str(&config::get_addr(cfg.application_out_socket_port)) {
            Ok(addr) => addr,
            Err(e) => panic!("{}", e),
        };

        let sock = UdpSocket::bind(addr).unwrap();

        let addr_out = match SocketAddrV4::from_str(&config::get_addr(cfg.application_out_port)) {
            Ok(addr) => addr,
            Err(e) => panic!("{}", e),
        };

        OSCClient {
            socket: sock,
            out_addr: addr_out
        }
    }


    pub fn send(&self, packet: OscPacket) {
        let _ = self.socket.send_to(&encoder::encode(&packet).unwrap(), self.out_addr);
    }
}
