#![allow(unused_imports)]

extern crate memprocreader;

use std::time::{Instant};
use std::io;
use std::io::Read;
use std::io::Write;

use memprocreader::ProcessLocker;


fn main() {
    let region = std::env::args().nth(1).expect("Provide region (stack|heap)");
    let display = std::env::args().nth(2).expect("Provide display (hex|bin)");
    let pid: i32 = std::env::args().nth(3).expect("Provide pid").parse().expect("Not a number");

    let data = 
    {
        let lock = ProcessLocker::lock(pid).expect(&format!("Can't lock {}", pid));


        match region.as_str() {
            "stack" => lock.stack().expect("Can't dump stack"),
            "heap" => lock.heap().expect("Can't dump heap"),
            _ => panic!("region must be 'stack' or 'heap'"),
        }
    };


    match display.as_str() {
        "hex" => {
            println!("Size: {}B", data.len());

            let mut out = String::new();
            for (offset, v) in data.chunks(8).enumerate() {
                out.push_str(&format!("0x{:x?}: {:x?}\n", offset, v));
            }
            println!("{}", &out);
        },
        "bin" => {
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            handle.write_all(&data).unwrap();
        },
        _ => panic!("Display must be \"hex\" or \"bin\""),
    }
}
