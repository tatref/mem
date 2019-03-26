use std::str::FromStr;
use std::fs::File;
use std::io::BufReader;
use std::io::{self,prelude::*};
use std::fmt;
use std::time::{Duration, Instant};
use regex;


/// http://man7.org/linux/man-pages/man5/proc.5.html
/// https://unix.stackexchange.com/questions/6301/how-do-i-read-from-proc-pid-mem-under-linux


#[derive(Debug, Copy, Clone)]
struct Address {
    start: u64,
    end: u64,
}

impl FromStr for Address {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut address = s.split('-')
            .map(|addr| u64::from_str_radix(addr, 16).unwrap());
        let start = address.next().ok_or_else(|| ())?;
        let end = address.next().ok_or_else(|| ())?;

        Ok(Address { start, end })
    }
}


#[derive(Debug, Copy, Clone)]
struct Permissions {
    r: bool,
    w: bool,
    x: bool,
    s: bool,
    p: bool,
}

impl FromStr for Permissions {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut input = input.bytes();

        let r = input.next().unwrap() != b'-';
        let w = input.next().unwrap() != b'-';
        let x = input.next().unwrap() != b'-';

        let last = input.next().unwrap();
        let (s, p) = match last {
            b's' => (true, false),
            b'p' => (false, true),
            _ => (false, false),
        };

        Ok(Permissions { r, w, x, s, p })
    }
}


#[derive(Debug, Copy, Clone)]
struct Device {
    major: u32,
    minor: u32,
}

impl FromStr for Device {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut input = input.split(':');

        let major: u32 = input.next().unwrap().parse().unwrap();
        let minor: u32 = input.next().unwrap().parse().unwrap();

        Ok(Device { major, minor })
    }
}


#[derive(Debug, Clone)]
struct MappedMemoryRegion {
    address: Address,
    permissions: Permissions,
    offset: u64,
    device: Device,
    inode: u64,
    pathname: Option<String>,
}

impl FromStr for MappedMemoryRegion {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut input = input.split_whitespace();

        let address: Address = input.next().unwrap().parse().unwrap();
        let permissions: Permissions = input.next().unwrap().parse().unwrap();
        let offset: u64 = u64::from_str_radix(input.next().unwrap(), 16).unwrap();
        let device: Device = input.next().unwrap().parse().unwrap();
        let inode: u64 = input.next().unwrap().parse().unwrap();
        let pathname: Option<String> = input.next().map(|x| x.to_owned());

        Ok(MappedMemoryRegion{ address, permissions, offset, device, inode, pathname })
    }
}

struct MappedMemoryRegions {
    maps: Vec<MappedMemoryRegion>,
}

impl MappedMemoryRegions {
    fn from_pid(pid: u32) -> Self {
        let mut maps_reader = BufReader::new(File::open(format!("/proc/{}/maps", pid))
                                             .expect("Unable to open maps file"));
        let mut mem_maps = String::new();
        maps_reader.read_to_string(&mut mem_maps).expect("Unable to read maps file");

        let maps: Vec<_> = mem_maps.lines()
            .map(|line| {
                line.parse::<MappedMemoryRegion>().expect("Unable to parse maps file")
            })
        .collect();


        MappedMemoryRegions { maps }
    }
}


fn main() {
    let pid: u32 = std::env::args().nth(1).expect("Provide pid").parse().expect("Provide pid");

    let mapped_memory_regions = MappedMemoryRegions::from_pid(pid);

    let mut mem = File::open(format!("/proc/{}/mem", pid)).unwrap();


    //let re = regex::bytes::Regex::new(r"(?-u)(?P<cstr>[^\x00]+)\x00").unwrap();
    let re = regex::bytes::Regex::new(r"(?-u)(?P<cstr>[\x20-\x7f]{3,})\x00").unwrap();

    let mut total = 0;
    for mapped_memory_region in mapped_memory_regions.maps.iter() {

        if mapped_memory_region.permissions.r && mapped_memory_region.pathname.as_ref().map(|x| x.as_ref()) != Some("[vvar]") {
            let instant = Instant::now();

            println!("{:#?}", mapped_memory_region);
            //println!("mem: {:x}-{:x}", mapped_memory_region.address.start, mapped_memory_region.address.end);
            println!("size= {}", mapped_memory_region.address.end - mapped_memory_region.address.start);

            let mem_size = mapped_memory_region.address.end - mapped_memory_region.address.start;
            total += mem_size;
            println!("running total={}", total);

            let mut mem = mem.try_clone().unwrap();
            mem.seek(io::SeekFrom::Start(mapped_memory_region.address.start)).unwrap();
            //let content: Result<Vec<_>, _>  = mem.take(mem_size).bytes().collect();
            //let content = content.expect("read mem failed");
            let mut data = vec![0u8; mem_size as usize];
            mem.read_exact(&mut data).unwrap();

            //println!("{:?}", data);
            //println!("content_len={}", content.len());

            let elapsed = instant.elapsed().as_millis() as f32 / 1000.;

            println!("elapsed={:?}", elapsed);
            println!("");

            let strings: Vec<_> = re.captures_iter(&data)
                .map(|c| String::from_utf8_lossy(c.name("cstr").unwrap().as_bytes()).to_owned())
                //.map(|c| c.name("cstr").unwrap().as_bytes())
                .collect();
            println!("{:?}", strings);

            //for x in data.windows(5)
            //    .filter(|x| { x == b"\x00\x01\x02\x03\x04"}) {
            //        println!("FOUND: {:?} in {:?}", x, mapped_memory_region.pathname);
            //    }

            //for x in data.windows(5)
            //    .filter(|x| { x == b"\x10\x11\x12\x13\x14"}) {
            //        println!("FOUND: {:?} in {:?}", x, mapped_memory_region.pathname);
            //    }

        }
    }



}
