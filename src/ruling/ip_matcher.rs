extern crate treebitmap;

use std::net::Ipv4Addr;
use std::sync::Arc;
use std::collections::HashMap;

use std::fs;
use std::fs::DirEntry;
use std::io;
use std::io::Read;
use std::str::FromStr;

use self::treebitmap::IpLookupTable;
use self::treebitmap::IpLookupTableOps;
use super::util::find_confs;
use std::path;

pub struct IpMatcher {
    ip4_table: IpLookupTable<Ipv4Addr, Arc<String>>
}

impl IpMatcher {
    pub fn new(confpath: &path::Path) -> io::Result<IpMatcher> {
        let regions = find_confs(confpath, "ipregion")?;
        Ok(IpMatcher{
            ip4_table: build_ip4_table(&regions)
        })
    }

    #[allow(dead_code)]
    pub fn rule_ip4(&self, ip: Ipv4Addr) -> Option<&Arc<String>> {
        match self.ip4_table.longest_match(ip) {
            Some((_, _, v)) => Some(v),
            None => None,
        }
    }
}

fn build_ip4_table(regions: &HashMap<Arc<String>, Vec<DirEntry>>)
    -> IpLookupTable<Ipv4Addr, Arc<String>> {
    let mut i4table= IpLookupTable::new();
    for (region, conf) in regions {
        for entry in conf.iter() {
            let f = fs::File::open(entry.path()).unwrap();
            let mut bufreader = io::BufReader::new(f);
            let mut contents = String::new();
            bufreader.read_to_string(&mut contents).unwrap();
            for line in contents.lines() {
                if line.len() == 0 || line.starts_with('#') { continue }
                let ip4 = line.split_whitespace().next();
                if let Some(ip) = ip4 {
                    let mut p = ip.splitn(2, '/');
                    let a = p.next().unwrap();
                    let m = p.next().unwrap();
                    let m = u32::from_str(m).unwrap();
                    let a = Ipv4Addr::from_str(a).unwrap();
                    i4table.insert(a, m, region.clone());
                }
            }
        }
    }
    i4table
}
