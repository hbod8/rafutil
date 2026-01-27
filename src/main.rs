use std::fs::File;
use std::io::{Read, Seek};
use std::{env, io};
use crate::binary::{BufBinaryReader, Endian, FromBinary};

mod exif;
mod raf;
mod binary;

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut source = File::open(args.get(2).expect("File not found")).expect("Unable to open file");

    let mut reader = BufBinaryReader::new(&mut source, Endian::Big);

    let parsed = raf::Raf::read_from(&mut reader).unwrap();

    println!("{}", parsed);
}
