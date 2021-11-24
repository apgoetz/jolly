// contains logic to parse the saved links for jolly
// basic format of jolly storage: 
//
// ['filename.txt'] # filename or name for bookmark, can also contain path
// tags = ['foo', 'text', 'baz']
// location = '/optional/path/to/filename'
// location = 'http://example.com' #can contain mozilla style query string (single %s)
// keyword = 'k' # keyword used for mozilla style query strings
use std::fs;
use std::path::Path;
use toml;

struct StoreEntry {
    filename : String,
    location : String,
    tags : Vec<String>,
}

struct Store;

fn load_store<P:AsRef<Path>>(path : P) -> Option<Store> {
    parse_store(&fs::read_to_string(path).ok()?)
}

fn parse_store(text : &str) -> Option<Store> {
    let _value = toml::from_str::<toml::Value>(text).ok()?;
    None
}
