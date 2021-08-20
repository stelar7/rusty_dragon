use std::thread::sleep;

#[path = "rman/RMANFile.rs"]
mod rmanfile;

fn main() {
    let data = std::fs::read("C:\\cdragon\\cdragon\\rman\\DC9F6F78A04934D6.manifest").unwrap();
    let result = rmanfile::parse(&data);
    println!("{:?}", result);
}
