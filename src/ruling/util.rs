use std::io::{self};
use std::fs::{self, DirEntry};
use std::sync::Arc;
use std::collections::HashMap;
use std::path;
use bytes::Bytes;

pub fn find_confs(path: &path::Path, kind: &str)-> io::Result<HashMap<Bytes, Vec<DirEntry>>>{
    let mut region_map = HashMap::new();
    for entry in fs::read_dir(path)? {
        let file = entry?;
        let ftype = file.file_type()?;

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
    let rest = filename.trim_left_matches(prefix);
    if !rest.starts_with(".") { return None; }
    let rest = rest.trim_left_matches(".");
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
