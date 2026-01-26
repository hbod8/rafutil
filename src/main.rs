use std::fs::File;
use std::io::{Read, Seek};
use std::{env, io};

mod exif;
mod raf;

pub trait FromBinary: Sized {
    fn read_from<R: Read + Seek>(reader: &mut R) -> io::Result<Self>;
}

// @TODO: Swapping to a buffered reader that has a specified capacity would be a better, more performant implementation here.
pub trait FromBinaryLimit: Sized {
    fn read_from_to_limit<R: Read + Seek>(reader: &mut R, limit: u64) -> io::Result<Self>;
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut source = File::open(args.get(2).expect("File not found")).expect("Unable to open file");

    let parsed = raf::Raf::read_from(&mut source).unwrap();

    println!("{}", parsed);
}
