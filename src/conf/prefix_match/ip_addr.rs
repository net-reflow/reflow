extern crate treebitmap;

use std::net::Ipv4Addr;
use std::collections::HashMap;

use std::fs;
use std::fs::DirEntry;
use std::str::FromStr;
use std::str::from_utf8;

use bytes::Bytes;
use failure::Error;
use treebitmap::IpLookupTable;
use super::util::find_addr_map_files;
use std::path;
use std::net::IpAddr;

pub struct IpMatcher {
    ip4_table: IpLookupTable<Ipv4Addr, Bytes>
}

impl IpMatcher {
    pub fn new(confpath: &path::Path) -> Result<IpMatcher, Error> {
        let regions = find_addr_map_files(confpath)?;
        Ok(IpMatcher{
            ip4_table: build_ip4_table(&regions)?,
        })
    }

    #[allow(dead_code)]
    pub fn rule_ip4(&self, ip: Ipv4Addr) -> Option<&Bytes> {
        match self.ip4_table.longest_match(ip) {
            Some((_, _, v)) => Some(v),
            None => None,
        }
    }

    pub fn match_ip(&self, ip: IpAddr)-> Option<Bytes> {
        match ip {
            IpAddr::V4(i) => self.ip4_table.longest_match(i)
                .map(|(_i, _m, v)| v.clone()),
            IpAddr::V6(_i) => None,
        }
    }
}

fn build_ip4_table(regions: &HashMap<Bytes, Vec<DirEntry>>)
    -> Result<IpLookupTable<Ipv4Addr, Bytes>, Error> {
    let mut i4table= IpLookupTable::new();
    for (region, conf) in regions {
        for entry in conf.iter() {
            let  contents = fs::read(entry.path())?;
            let ls = contents.split(|&x| x == b'\r' || x == b'\n');
            for line in ls {
                if line.len() == 0 || line.starts_with(b"#") { continue }
                let ip4 = line.split(|x| x.is_ascii_whitespace()).next();
                if let Some(ip) = ip4 {
                    let mut p = ip.splitn(2, |&x| x == b'/');
                    let a = p.next().ok_or_else(|| format_err!("Not address"))?;
                    let m = p.next().ok_or_else(|| format_err!("Not masklen"))?;
                    let a = from_utf8(a)?;
                    let a = Ipv4Addr::from_str(a)?;
                    let m = from_utf8(m)?;
                    let m = u32::from_str(m)?;
                    i4table.insert(a, m, region.clone());
                }
            }
        }
    }
    Ok(i4table)
}
