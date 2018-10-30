extern crate treebitmap;

use std::net::Ipv4Addr;

use std::fs;
use std::str::FromStr;
use std::str::from_utf8;

use bytes::Bytes;
use failure::Error;
use treebitmap::IpLookupTable;
use super::util::find_addr_map_files;
use std::path;
use std::net::IpAddr;
use super::util::lines_without_comments;
use util::BsDisp;
use std::net::Ipv6Addr;

pub struct IpMatcher {
    ip4_table: IpLookupTable<Ipv4Addr, Bytes>,
    ip6_table: IpLookupTable<Ipv6Addr, Bytes>,
}

impl IpMatcher {
    pub fn new(confpath: &path::Path) -> Result<IpMatcher, Error> {
        let regions = find_addr_map_files(confpath)?;

        let mut i4table= IpLookupTable::new();
        let mut i6table= IpLookupTable::new();
        for (region, conf) in regions {
            for entry in conf.iter() {
                let  contents = fs::read(entry.path())?;
                let ls = lines_without_comments(&contents);
                for line in ls {
                    let (a,m) = try_parse_ip_network(line)
                        .map_err(|e| format_err!(
                    "Can't parse {} as IP network: {:?}", BsDisp::new(line), e))?;
                    match a {
                        IpAddr::V6(a) => i6table.insert(a, m, region.clone()),
                        IpAddr::V4(a) => i4table.insert(a, m, region.clone()),
                    };
                }
            }
        }
        Ok(IpMatcher{
            ip4_table: i4table,
            ip6_table: i6table,
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
            IpAddr::V6(i) => self.ip6_table.longest_match(i)
                .map(|(_i, _m, v)| v.clone()),
        }
    }
}

fn try_parse_ip_network(line: &[u8])-> Result<(IpAddr, u32), Error> {
    let mut p = line.splitn(2, |&x| x == b'/');
    let a = p.next().ok_or_else(|| format_err!("Not address"))?;
    let m = p.next().ok_or_else(|| format_err!("Not masklen"))?;
    let a = from_utf8(a)?;
    let a = Ipv4Addr::from_str(a)
        .map(|a| IpAddr::V4(a))
        .or_else(|_e| {
            Ipv6Addr::from_str(a).map(|a| IpAddr::V6(a))
        })?;
    let m = from_utf8(m)?;
    let m = u32::from_str(m)?;
    Ok((a, m))
}

#[cfg(test)]
mod tests {
    use std::path;
    use std::fs;
    use super::IpMatcher;
    use bytes::Bytes;
    use std::net::IpAddr;
    use std::str::FromStr;

    #[test]
    fn test_some_addresses() {
        let p = path::PathBuf::from("test/conf.d");
        let f = fs::read_to_string(p.join("addrzone-expectation")).unwrap();
        let r =
            f.lines().filter_map(|l: &str|-> Option<(IpAddr, Option<Bytes>)> {
                let l = l.trim();
                if l.len() == 0 {
                    None
                } else {
                    let v: Vec<&str> = l.trim().split_whitespace().collect();
                    let a = IpAddr::from_str(v[0]).unwrap();
                    let r = v.get(1).map(|&x| x.into());
                    Some((a, r))
                }
            });
        let d = IpMatcher::new(&p).unwrap();
        for (h, v) in r {
            let j = d.match_ip(h);
            assert_eq!(j, v, "testing address {:?}", h);
        }
    }
}