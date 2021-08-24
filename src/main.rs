#[path = "rman/rman.rs"]
mod rman;

extern crate serde_json;

#[macro_use]
extern crate serde_derive;

fn main() {
    let data = std::fs::read("C:\\cdragon\\cdragon\\rman\\DC9F6F78A04934D6.manifest").unwrap();
    let result = rman::parse(&data);

    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}
