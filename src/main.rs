#[path = "rman/rman.rs"]
mod rman;

fn main() {
    let data = std::fs::read("C:\\cdragon\\cdragon\\rman\\DC9F6F78A04934D6.manifest").unwrap();
    let result = rman::parse(&data);
    // println!("{:?}", result);
}
