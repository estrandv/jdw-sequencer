extern crate rosc;

use rosc::{OscPacket, decoder};
use std::env;
use std::net::{SocketAddr, SocketAddrV4, UdpSocket};
use std::str::FromStr;
use std::sync::Arc;
use crate::OSCRead;
use rosc::encoder;

pub struct OSCClient {
    socket: UdpSocket,
    out_addr: SocketAddrV4,
    buf: [u8; 1536]
}

impl OSCClient {

    pub fn new() -> OSCClient {
        // TODO: Replace with config
        let addr = match SocketAddrV4::from_str("127.0.0.1:14447") {
            Ok(addr) => addr,
            Err(e) => panic!("{}", e),
        };

        let sock = UdpSocket::bind(addr).unwrap();

        // TODO: Replace with config
        let addr_out = match SocketAddrV4::from_str("127.0.0.1:14447") {
            Ok(addr) => addr,
            Err(e) => panic!("{}", e),
        };

        let buf = [0u8; rosc::decoder::MTU];

        OSCClient {
            socket: sock,
            out_addr: addr_out,
            buf
        }

    }

    pub fn poll(&mut self) -> Result<OscPacket, String> {
        return match self.socket.recv_from(&mut self.buf) {
            Ok((size, addr)) => {
                let (_, packet) = rosc::decoder::decode_udp(&self.buf[..size]).unwrap();
                Ok(packet)
            }
            Err(e) => {Err("Error receiving from osc socket".to_string())}
        };
    }

    pub fn send(&self, packet: OscPacket) {
        self.socket.send_to(&encoder::encode(&packet).unwrap(), self.out_addr);
    }
}

