extern crate radix_trie;

use std::sync::Arc;
use std::collections::HashMap;

use std::fs;
use std::fs::DirEntry;
use std::io;
use std::io::Read;
use bytes::Bytes;

use super::util::find_confs;
use self::radix_trie::Trie;
use std::path;

pub struct DomainMatcher {
    domain_trie: Trie<Vec<u8>, Bytes>,
}


impl <'a> DomainMatcher {
    pub fn new(config: &path::Path) -> io::Result<DomainMatcher> {
        let regions = find_confs(config, "region")?;
        let ruler = DomainMatcher {
            domain_trie: build_domain_trie(&regions)?,
        };
        Ok(ruler)
    }

    pub fn rule_domain(&self, d: &[u8]) -> Option<Bytes> {
        if let Some(x) = self.domain_trie.get(d) {
            return Some(x.clone());
        }
        let d = split_off_last(d)?;
        self.rule_domain(d)
    }
}

fn build_domain_trie(regions: &HashMap<Bytes, Vec<DirEntry>>)
                     -> io::Result<Trie<Vec<u8>, Bytes>> {
    let mut trie= Trie::new();
    for (region, conf) in regions {
        for entry in conf.iter() {
            let contents  = fs::read(entry.path())?;
            let mut ns: Vec<&[u8]> = vec![];

            for line in contents.split(|&x| x == b'\r' || x == b'\n') {
                if line.len() == 0 || line.starts_with(b"#") { continue }
                let domain = line.split(|x| x.is_ascii_whitespace()).next();
                if let Some(d) = domain {
                    let mut ds: Vec<&[u8]> = d.split(|&x| x==b'.').collect();
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
                    let d = ds.join(&b'.');
                    trie.insert(d, region.clone());
                }
            }
        }
    }
    Ok(trie)
}

/// remove the last part of a domain
fn split_off_last(d: &[u8]) -> Option<&[u8]> {
    let l = d.len();
    for i in (0..l) {
        let j = l - i;
        if d[j] == b'.' {
            return Some(&d[0..j]);
        }
    }
    return None;
}