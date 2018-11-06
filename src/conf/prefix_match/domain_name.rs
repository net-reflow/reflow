extern crate radix_trie;

use std::collections::HashMap;

use std::fs;
use std::fs::DirEntry;
use std::io;
use bytes::Bytes;
use failure::Error;

use super::util::find_domain_map_files;
use self::radix_trie::Trie;
use std::path;
use util::BsDisp;
use super::util::lines_without_comments;
use std::time::Instant;

pub struct DomainMatcher {
    domain_trie: Trie<Vec<u8>, Bytes>,
}


impl <'a> DomainMatcher {
    pub fn new(config: &path::Path) -> Result<DomainMatcher, Error> {
        let now = Instant::now();
        let regions = find_domain_map_files(config)?;
        check_zone_name(regions.keys().collect())?;
        let (trie, count) = build_domain_trie(&regions)?;
        let ruler = DomainMatcher {
            domain_trie: trie,
        };
        let elapse = now.elapsed().as_secs() * 1000 + now.elapsed().subsec_millis() as u64;
        info!("Loaded {} domain name prefixes in {}ms", count, elapse);
        Ok(ruler)
    }

    /// argument starts with the root, such as com.google.www
    pub fn rule_domain(&self, d: &[u8]) -> Option<Bytes> {
        if let Some(x) = self.domain_trie.get(d) {
            return Some(x.clone());
        }
        let d = split_off_last(d)?;
        self.rule_domain(d)
    }
}

fn build_domain_trie(regions: &HashMap<Bytes, Vec<DirEntry>>)
                     -> io::Result<(Trie<Vec<u8>, Bytes>, u32)> {
    let mut count: u32 = 0;
    let mut trie= Trie::new();
    for (region, conf) in regions {
        for entry in conf.iter() {
            let contents  = fs::read(entry.path())?;
            let ls = lines_without_comments(&contents);
            let mut ns: Vec<&[u8]> = vec![];

            for d in ls {
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
                count += 1;
            }
        }
    }
    Ok((trie, count))
}

/// remove the last part of a domain
fn split_off_last(d: &[u8]) -> Option<&[u8]> {
    let l = d.len();
    for i in 0..l {
        let j = l -1- i;
        if d[j] == b'.' {
            return Some(&d[0..j]);
        }
    }
    return None;
}

#[cfg(test)]
mod tests {
    use std::path;
    use std::fs;
    use super::DomainMatcher;
    use bytes::Bytes;

    #[test]
    fn test_some_domains() {
        let p = path::PathBuf::from("test/conf.d");
        let f = fs::read_to_string(p.join("namezone-expectation")).unwrap();
        let r =
            f.lines().map(|l|-> (&str, Option<Bytes>) {
                let v: Vec<&str> = l.trim().split_whitespace().collect();
                let d = v[0];
                let r = v.get(1).map(|&x| x.into());
                (d, r)
            });
        let d = DomainMatcher::new(&p).unwrap();
        for (h, v) in r {
            let j = d.rule_domain(h.as_bytes());
            assert_eq!(j, v, "testing {}", h);
        }
    }
}

fn check_zone_name(ks: Vec<&Bytes>)->Result<(), Error> {
    let reserved = vec!["else"];
    for k in ks {
        for i in &reserved {
            if k == i.as_bytes() {
                return Err(format_err!("{} can't be used the name of a domain name zone",
                                  BsDisp::new(&k),
                ));
            }
        }
    }
    Ok(())
}