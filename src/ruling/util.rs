use std::io::{self};
use std::fs::{self, DirEntry};
use std::sync::Arc;
use std::collections::HashMap;

pub fn find_confs(path: &str, kind: &str)-> io::Result<HashMap<Arc<String>, Vec<DirEntry>>>{
    let mut region_map = HashMap::new();
    let kindprefix = format!("{}.", kind);
    for entry in fs::read_dir(path)? {
        let file = entry?;
        let ftype = file.file_type()?;

        let name = file.file_name();
        let name= name.to_string_lossy();
        if ftype.is_dir() && name.starts_with(&kindprefix) {
            let region_name = name.trim_left_matches(&kindprefix).to_owned();
            let conf = find_dir_entris(file)?;
            region_map.insert(Arc::new(region_name), conf);
        }
    }
    Ok(region_map)
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
