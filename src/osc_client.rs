extern crate rosc;

use std::net::{SocketAddr, SocketAddrV4, UdpSocket};
use std::str::FromStr;

use rosc::{OscPacket};
use rosc::encoder;
use crate::config;
use crate::config::{APPLICATION_IN_PORT, APPLICATION_OUT_PORT, APPLICATION_OUT_SOCKET_PORT};

const BUFFER_SIZE: usize = 33072;

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
        self.socket.send_to(&encoder::encode(&packet).unwrap(), self.out_addr);
    }
}

pub struct OSCPoller {
    socket: UdpSocket,
    buf: [u8; BUFFER_SIZE]
}

impl OSCPoller {

    pub fn new() -> OSCPoller {
        let addr = match SocketAddrV4::from_str(&config::get_addr(APPLICATION_IN_PORT)) {
            Ok(addr) => addr,
            Err(e) => panic!("{}", e),
        };

        let sock = UdpSocket::bind(addr).unwrap();

        //let buf = [0u8; rosc::decoder::MTU];
        // TODO: Compare with size in struct declaration (should be same value)
        // THe MTU constant is way too low... I think.
        // Too low results in parts of large packets being dropped before receiving 
        // Heck, might just be some kind of buffer thing where I'm supposed to read 
        // multiple things but only end up reading the first.. . 
        let buf = [0u8; BUFFER_SIZE];

        OSCPoller {
            socket: sock,
            buf
        }

    }

    pub fn poll(&mut self) -> Result<OscPacket, String> {
        return match self.socket.recv_from(&mut self.buf) {
            Ok((size, _)) => {
                let (rem, packet) = rosc::decoder::decode_udp(&mut self.buf[..size]).unwrap();
                // TODO: Something going on here with rem - there will be a remnant after buf size completes 
                // Which I guess needs to be handled in those cases 
                Ok(packet)
            }
            Err(e) => {Err("Error receiving from osc socket".to_string())}
        };
    }

}

