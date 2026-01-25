use std::env;
use std::fs::File;
use raf::Raf;
use crate::raf::FromBinary;

mod raf;
mod exif;

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut source = File::open(args.get(2).expect("File not found")).expect("Unable to open file");

    let parsed = Raf::read_from(&mut source).unwrap();

    println!("{}", parsed);
}
