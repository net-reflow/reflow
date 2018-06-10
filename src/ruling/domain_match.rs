extern crate radix_trie;

use std::sync::Arc;
use std::collections::HashMap;

use std::fs;
use std::fs::DirEntry;
use std::io;
use std::io::Read;

use super::util::find_confs;
use self::radix_trie::Trie;
use std::path;

pub struct DomainMatcher {
    domain_trie: Trie<String, Arc<String>>,
}

impl <'a> DomainMatcher{
    pub fn new(config: &path::Path)-> io::Result<DomainMatcher> {
        let regions = find_confs(config, "region")?;
        let ruler = DomainMatcher {
            domain_trie: Self::build_domain_trie(&regions),
        };
        Ok(ruler)
    }

    pub fn rule_domain(&self, domain: &str) -> Option<&Arc<String>> {
        let mut d = domain.to_string();
        if !d.ends_with('.') { d.push('.'); }
        self.domain_trie.get_ancestor_value(&d)
    }

    fn build_domain_trie(regions: &HashMap<Arc<String>, Vec<DirEntry>>)
        -> Trie<String, Arc<String>> {
        let mut trie= Trie::new();
        for (region, conf) in regions {
            for entry in conf.iter() {
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
}
