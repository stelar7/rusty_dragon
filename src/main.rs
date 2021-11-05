mod macros;
mod rman;
mod wad;

#[macro_use]
extern crate serde_derive;
extern crate serde_json;

#[macro_use]
extern crate strum_macros;
extern crate strum;

fn main() {
    let data = std::fs::read("C:\\cdragon\\cdragon\\wad\\Ahri.wad.client").unwrap();
    let result = wad::parse(&data);
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
    /*
        let data = std::fs::read("C:\\cdragon\\cdragon\\rman\\DC9F6F78A04934D6.manifest").unwrap();
        let result = rman::parse(&data);
        println!("{}", serde_json::to_string_pretty(&result).unwrap());
    */
}
