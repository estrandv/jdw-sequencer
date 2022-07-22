extern crate rosc;

use std::net::{SocketAddr, SocketAddrV4, UdpSocket};
use std::str::FromStr;

use rosc::{OscPacket};
use rosc::encoder;
use crate::config;
use crate::config::{APPLICATION_IN_PORT, APPLICATION_OUT_PORT, APPLICATION_OUT_SOCKET_PORT};

pub struct OSCClient {
    socket: UdpSocket,
    out_addr: SocketAddrV4,
    buf: [u8; 1536]
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
            out_addr: addr_out,
            buf
        }
    }


    pub fn send(&self, packet: OscPacket) {
        self.socket.send_to(&encoder::encode(&packet).unwrap(), self.out_addr);
    }
}

pub struct OSCPoller {
    socket: UdpSocket,
    buf: [u8; 1536]
}

impl OSCPoller {

    pub fn new() -> OSCPoller {
        let addr = match SocketAddrV4::from_str(&config::get_addr(APPLICATION_IN_PORT)) {
            Ok(addr) => addr,
            Err(e) => panic!("{}", e),
        };

        let sock = UdpSocket::bind(addr).unwrap();

        let buf = [0u8; rosc::decoder::MTU];

        OSCPoller {
            socket: sock,
            buf
        }

    }

    pub fn poll(&mut self) -> Result<OscPacket, String> {
        return match self.socket.recv_from(&mut self.buf) {
            Ok((size, _)) => {
                let (_, packet) = rosc::decoder::decode_udp(&self.buf[..size]).unwrap();
                Ok(packet)
            }
            Err(e) => {Err("Error receiving from osc socket".to_string())}
        };
    }

}

