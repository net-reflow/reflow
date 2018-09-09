extern crate radix_trie;
extern crate treebitmap;

use self::radix_trie::Trie;
use self::treebitmap::IpLookupTable;
use self::treebitmap::IpLookupTableOps;

use std::collections::HashMap;
use std::fs;
use std::fs::DirEntry;
use std::io;
use std::io::Read;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::str::FromStr;

pub mod serve;

pub struct Ruler{
    domain_trie: Trie<String, Arc<String>>,
    ip4_table: IpLookupTable<Ipv4Addr, Arc<String>>
}

impl <'a> Ruler{
    pub fn new(config: &str)-> Ruler {
        let regions = Self::find_confs(config);
        let ruler = Ruler {
            domain_trie: Self::build_domain_trie(&regions),
            ip4_table: build_ip4_table(&regions)
        };

        ruler
    }

    pub fn rule_ip4(&self, ip: Ipv4Addr) -> Option<&Arc<String>> {
        match self.ip4_table.longest_match(ip) {
            Some((_, _, v)) => Some(v),
            None => None,
        }
    }

    pub fn rule_domain(&self, domain: &str) -> Option<&Arc<String>> {
        let mut d = domain.to_string();
        if !d.ends_with('.') { d.push('.'); }
        self.domain_trie.get_ancestor_value(&d)
    }

    fn build_domain_trie(regions: &HashMap<Arc<String>, RegionConfFiles>)
        -> Trie<String, Arc<String>> {
        let mut trie= Trie::new();
        for (region, conf) in regions {
            for entry in &conf.domain {
                let f = fs::File::open(entry.path()).unwrap();
                let mut bufreader = io::BufReader::new(f);
                let mut contents = String::new();
                bufreader.read_to_string(&mut contents).unwrap();
                let mut ns: Vec<&str> = vec![];
                for line in contents.lines() {
                    if line.len() == 0 || line.starts_with('#') { continue }
                    let domain = line.split_whitespace().next();
                    if let Some(domain) = domain {
                        let mut ds: Vec<&str> = domain.split('.').collect();
                        for i in 0..ds.len() {
                            if ds[i].len() == 0 {
                                ds[i] = ns[i];
                            } else {
                                if i < ns.len() {
                                    ns[i] = ds[i];
                                } else {
                                    assert_eq!(i,ns.len());
                                    ns.push(ds[i]);
                                }
                            }
                        }
                        let mut d = ds.join(".");
                        d.push('.');
                        trie.insert(d, region.clone());
                    }
                }
            }
        }
        trie
    }

    pub fn find_confs(config: &str)-> HashMap<Arc<String>, RegionConfFiles> {
        let mut region_map = HashMap::new();
        for entry in fs::read_dir(config).unwrap() {
            match entry {
                Ok(file) => {
                    match file.file_type() {
                        Ok(ftype) => {
                            let name = file.file_name();
                            let name = name.to_string_lossy();
                            if ftype.is_dir() && name.starts_with("region.")
                                && name.len() > 7 {
                                let region_name = name.trim_left_matches("region.").to_owned();
                                let conf = RegionConfFiles::new(file).unwrap();
                                region_map.insert(Arc::new(region_name), conf);
                            }
                        }
                        Err(e) => panic!("error reading dir entry type {:?}", e)
                    }
                }
                Err(e) => panic!("error in entry {:?}", e)
            }
        }
        region_map
    }

}

fn build_ip4_table(regions: &HashMap<Arc<String>, RegionConfFiles>)
        -> IpLookupTable<Ipv4Addr, Arc<String>> {
        let mut i4table= IpLookupTable::new();
        for (region, conf) in regions {
            for entry in &conf.ip4 {
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

#[derive(Debug)]
pub struct RegionConfFiles {
    domain: Vec<DirEntry>,
    ip4: Vec<DirEntry>,
    ip6: Vec<DirEntry>,
}

impl RegionConfFiles {
    fn new(dir: DirEntry)-> io::Result<RegionConfFiles> {
        let mut domain_files = vec![];
        let mut ip4_files = vec![];
        let mut ip6_files = vec![];
        let readdir = try!(fs::read_dir(dir.path()));
        for entry in readdir {
            let file = try!(entry);
            let file_type = try!(file.file_type());
            if file_type.is_file() || file_type.is_symlink() {
                let name = file.file_name();
                let name = name.to_string_lossy();
                if name.starts_with("domain") {
                    domain_files.push(file);
                } else if name.starts_with("ip4") {
                    ip4_files.push(file);
                } else if name.starts_with("ip6") {
                    ip6_files.push(file);
                }
            }
        }
        Ok(
            RegionConfFiles {
                domain: domain_files,
                ip4: ip4_files,
                ip6: ip6_files
            }
        )
    }
}
