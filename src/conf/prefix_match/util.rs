use std::io::{self};
use std::fs::{self, DirEntry};
use std::collections::HashMap;
use std::path;
use bytes::Bytes;

pub fn find_domain_map_files(config: &path::Path)->io::Result<HashMap<Bytes, Vec<DirEntry>>>{
    let p = config.join("namezone");
    let mut m = if p.exists() {
        find_map_files(&p)?
    } else {
        HashMap::new()
    };
    let m1 = find_confs(config, "region")?;
    merge_map_vec_value(&mut m, m1);
    Ok(m)
}
pub fn find_addr_map_files(config: &path::Path)->io::Result<HashMap<Bytes, Vec<DirEntry>>>{
    let p = config.join("addrzone");
    let mut m = if p.exists() {
        find_map_files(&p)?
    } else {
        HashMap::new()
    };
    let m1 = find_confs(config, "ipregion")?;
    merge_map_vec_value(&mut m, m1);
    Ok(m)
}
fn find_map_files(path: &path::Path)-> io::Result<HashMap<Bytes, Vec<DirEntry>>>{
    let mut m = HashMap::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let ftype = fs::metadata(entry.path())?.file_type();

        let n = entry.file_name();
        let name = n.to_str().expect("Bad encoding in filename");
        let nb: Bytes = name.into();
        if ftype.is_file() {
            m.insert(nb, vec![entry]);
        } else if ftype.is_dir() {
            m.insert(nb, find_dir_entris(entry)?);
        }
    }
    Ok(m)
}
/// merge two HashMaps whose values are both vectors
fn merge_map_vec_value(m0: &mut HashMap<Bytes, Vec<DirEntry>>, m1: HashMap<Bytes, Vec<DirEntry>>) {
    for (k,v) in m1.into_iter() {
        if m0.get(&k).is_some()  {
            m0.get_mut(&k).unwrap().extend(v);
        } else {
            m0.insert(k, v);
        }
    }
}
/// deprecated
fn find_confs(path: &path::Path, kind: &str)-> io::Result<HashMap<Bytes, Vec<DirEntry>>>{
    let mut region_map = HashMap::new();
    for entry in fs::read_dir(path)? {
        let file = entry?;
        let m = fs::metadata(file.path())?;
        let ftype = m.file_type();

        let f = file.file_name();
        let n = f.to_str().and_then(|x| extract_name(x, kind));
        if ftype.is_dir() && n.is_some() {
            let conf = find_dir_entris(file)?;
            region_map.insert(n.unwrap(), conf);
        }
    }
    Ok(region_map)
}

fn extract_name(filename: &str, prefix: &str)-> Option<Bytes> {
    if !filename.starts_with(prefix) { return None; }
    let rest = filename.trim_start_matches(prefix);
    if !rest.starts_with(".") { return None; }
    let rest = rest.trim_start_matches(".");
    if rest.len() < 1 { return  None; }
    Some(rest.into())
}

fn find_dir_entris(dir: DirEntry)-> io::Result<Vec<DirEntry>> {
    let readdir = fs::read_dir(dir.path())?;
    let entries = readdir.filter_map(|entry| {
        let file = entry.unwrap();
        let file_type = file.file_type().unwrap();
        if file_type.is_file() || file_type.is_symlink() {
            Some(file)
        } else {
            None
        }
    }).collect();
    Ok(entries)
}

pub fn lines_without_comments(bytes: &[u8])->impl Iterator<Item=&[u8]> {
    bytes
        .split(|&x| x == b'\r' || x == b'\n')
        .map(|line: &[u8]| {
        line.split(|&x| x == b'#').next().unwrap_or(b"")
            .split(|x| x.is_ascii_whitespace()).next().unwrap_or(b"")
    }).filter(|l| l.len() > 0)
}