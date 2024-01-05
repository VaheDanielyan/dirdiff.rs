use std::path::{PathBuf, Path};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;

use walkdir::WalkDir;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use md5;

#[derive(Debug, Clone)]
pub enum Hash {
    Valid { hash: String },
    Invalid { error: String },
}

impl Hash {
    pub fn new(path: &Path) -> Hash {
        let mut file = match File::open(path) {
            Ok(file) => file,
            Err(e) => return Hash::Invalid { error: e.to_string() }
        };

        let mut buffer = Vec::new();
        match file.read_to_end(&mut buffer) {
            Err(e) => return Hash::Invalid { error: e.to_string() },
            _ => {}
        }
        let digest = md5::compute(&buffer);

        Hash::Valid { hash: format!("{:?}", digest)}
    }
}

#[derive(Debug)]
struct FileInfo {
    hash: Hash,
}

impl FileInfo {
    pub fn get_hash(&self) -> Hash {
        self.hash.clone()
    }
}

pub struct CmpResult {
    pub only_in_a : Vec<PathBuf>,
    pub only_in_b: Vec<PathBuf>,
    pub differs : Vec<PathBuf>,
}

impl CmpResult {
    pub fn new() -> CmpResult {
        CmpResult { only_in_a: Vec::new(),
                    only_in_b: Vec::new(),
                    differs: Vec::new() }
    }
}

pub fn dirdiff(path_a: &PathBuf, path_b: &PathBuf) -> CmpResult {
    let path_a_clone = path_a.clone();
    let path_b_clone = path_b.clone();

    let thread_a = std::thread::spawn(move || {
        return process_directory(&path_a_clone);
    });

    let thread_b = std::thread::spawn(move || {
        return process_directory(&path_b_clone);
    });
    
    let map1 : HashMap<PathBuf, FileInfo> = thread_a.join().unwrap().ok().unwrap();
    let map2 : HashMap<PathBuf, FileInfo> = thread_b.join().unwrap().ok().unwrap();
    let mut result : CmpResult = CmpResult::new();

    for item in &map1 {
        if map2.contains_key(item.0) {
            let item2 = map2.get(item.0).unwrap();
            let hash1 = match item.1.get_hash() {
                Hash::Valid { hash } => hash,
                Hash::Invalid { error } => error,
            };
            let hash2 = match item2.get_hash() {
                Hash::Valid { hash } => hash,
                Hash::Invalid { error } => error,
            };
            if hash1 != hash2 {
                result.differs.push(item.0.clone());
            }
        }
        else {
            result.only_in_a.push(item.0.clone());
        }
    }
    for item in &map2 {
        if !map1.contains_key(item.0) {
            result.only_in_b.push(item.0.clone());
        }
    }
    result
}

fn process_directory(path: &PathBuf) -> Result<HashMap<PathBuf, FileInfo>, String> {
    let files : Vec<PathBuf> = WalkDir::new(path)
        .into_iter()
        .filter_map(|f| f.ok())
        .filter(|f| f.file_type().is_file())
        .map(|f| f.path().to_owned())
        .collect();
    let result_map : HashMap<PathBuf, FileInfo> = files
        .par_iter()
        .map(|f| {
           let file_hash = Hash::new(f);
           (f.strip_prefix(path).unwrap().to_owned(), FileInfo { hash: file_hash })
        })
        .collect();
    Ok(result_map)
}
