extern crate rosc;

use std::net::{SocketAddrV4, UdpSocket};
use std::str::FromStr;

use rosc::{OscPacket};
use rosc::encoder;
use crate::config;
use crate::config::{APPLICATION_OUT_PORT, APPLICATION_OUT_SOCKET_PORT};


/*

    OSC I/O - as generic as possible.

*/


const BUFFER_SIZE: usize = 333072;

pub struct OSCClient {
    socket: UdpSocket,
    out_addr: SocketAddrV4,
}

impl OSCClient {
    pub fn new() -> OSCClient {
        let addr = match SocketAddrV4::from_str(&config::get_addr(APPLICATION_OUT_SOCKET_PORT)) {
            Ok(addr) => addr,
            Err(e) => panic!("{}", e),
        };

        let sock = UdpSocket::bind(addr).unwrap();
        let buf = [0u8; rosc::decoder::MTU];

        let addr_out = match SocketAddrV4::from_str(&config::get_addr(APPLICATION_OUT_PORT)) {
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
