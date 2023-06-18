use super::error::Result;
use std::net::IpAddr;

pub fn str_to_ip(addr: &str) -> Result<IpAddr> {
    let split = addr
        .splitn(4, '.')
        .map(|v| v.parse())
        .collect::<Result<Vec<u8>, _>>()?;

    Ok([split[0], split[1], split[2], split[3]].into())
}
