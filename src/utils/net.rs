use std::{
    io,
    net::{IpAddr, UdpSocket},
};

pub fn get_local_addr() -> io::Result<(String, u16)> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect("1.1.1.1:80")?;

    let local_addr = socket.local_addr()?;
    match local_addr.ip() {
        IpAddr::V4(ip) => Ok((ip.to_string(), local_addr.port())),
        IpAddr::V6(ip) => Ok((ip.to_string(), local_addr.port())),
    }
}
