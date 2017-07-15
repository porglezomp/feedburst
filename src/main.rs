#[macro_use]
extern crate nom;

use std::io::Read;

use nom::IResult;

mod parser;
mod feed;

fn main() {
    let fname = match std::env::args().nth(1) {
        Some(fname) => fname,
        None => {
            println!("Please pass a filename");
            return;
        }
    };

    let mut file = match std::fs::File::open(fname) {
        Ok(file) => file,
        Err(err) => {
            println!("Error opening file {}", err);
            return;
        }
    };

    let mut text = String::new();
    file.read_to_string(&mut text)
        .expect("IO Error");

    match parser::config(&text) {
        IResult::Done(_, feeds) => {
            for feed in feeds {
                println!("{:?}", feed);
            }
        }
        IResult::Error(err) => {
            println!("Error parsing config: {}", err);
        }
        IResult::Incomplete(..) => {
            unreachable!();
        }
    }
}

