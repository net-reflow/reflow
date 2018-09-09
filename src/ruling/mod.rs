extern crate radix_trie;

use self::radix_trie::Trie;

use std::collections::HashMap;
use std::fs;
use std::fs::DirEntry;
use std::io;
use std::io::Read;
use std::rc::Rc;

pub struct Ruler{
    domain_trie: Trie<String, Rc<String>>,
}

impl <'a> Ruler{
    pub fn new(config: &str)-> Ruler {
        let regions = Self::find_confs(config);
        let ruler = Ruler {
            domain_trie: Self::build_domain_trie(&regions),
        };

        ruler
    }

    pub fn rule_domain(&self, domain: &str) -> Option<&Rc<String>> {
        println!("{:?}", domain);
        let mut d = domain.to_string();
        if !d.ends_with('.') { d.push('.'); }
        self.domain_trie.get_ancestor_value(&d)
    }

    fn build_domain_trie(regions: &HashMap<Rc<String>, RegionConfFiles>)
        -> Trie<String, Rc<String>> {
        let mut trie= Trie::new();
        for (region, conf) in regions {
            for entry in &conf.domain {
                let f = fs::File::open(entry.path()).unwrap();
                let mut bufreader = io::BufReader::new(f);
                let mut contents = String::new();
                bufreader.read_to_string(&mut contents).unwrap();
                for line in contents.lines() {
                    if line.len() == 0 || line.starts_with('#') { continue }
                    let domain = line.split_whitespace().next();
                    if let Some(domain) = domain {
                        let mut d = domain.to_string();
                        if !d.ends_with('.') { d.push('.'); }
                        trie.insert(d, region.clone());
                    }
                }
            }
        }
        trie
    }

    pub fn find_confs(config: &str)-> HashMap<Rc<String>, RegionConfFiles> {
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
                                region_map.insert(Rc::new(region_name), conf);
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
