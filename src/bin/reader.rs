#![allow(unused_imports)]

use std::str::FromStr;
use std::fs::File;
use std::io::BufReader;
use std::io::{self,prelude::*};
use std::time::Duration;
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

        let major: u32 = u32::from_str_radix(input.next().unwrap(), 16).unwrap();
        let minor: u32 = u32::from_str_radix(input.next().unwrap(), 16).unwrap();

        Ok(Device { major, minor })
    }
}


#[derive(Debug, Clone)]
struct MappedMemoryRegionMetadata {
    address: Address,
    permissions: Permissions,
    offset: u64,
    device: Device,
    inode: u64,
    pathname: Option<String>,
}

impl FromStr for MappedMemoryRegionMetadata {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut input = input.split_whitespace();

        let address: Address = input.next().unwrap().parse().unwrap();
        let permissions: Permissions = input.next().unwrap().parse().unwrap();
        let offset: u64 = u64::from_str_radix(input.next().unwrap(), 16).unwrap();
        let device: Device = input.next().unwrap().parse().unwrap();
        let inode: u64 = input.next().unwrap().parse().unwrap();
        let pathname: Option<String> = input.next().map(|x| x.to_owned());

        Ok(MappedMemoryRegionMetadata{ address, permissions, offset, device, inode, pathname })
    }
}

struct ProcessLocker {
    pid: i32,
    mem_file: File,
}

impl ProcessLocker {
    fn lock(pid: i32) -> Result<Self, ()> {
        if unsafe { libc::kill(pid, libc::SIGSTOP)} != 0 {
            return Err(());
        }

        let mem_file = File::open(format!("/proc/{}/mem", pid)).unwrap();

        Ok(ProcessLocker { pid, mem_file })
    }

    fn get_maps(&self) -> Result<Vec<MappedMemoryRegionMetadata>, ()> {
        let mut maps_reader = BufReader::new(File::open(format!("/proc/{}/maps", self.pid))
                                             .expect("Unable to open maps file"));
        let mut mem_maps = String::new();
        maps_reader.read_to_string(&mut mem_maps).expect("Unable to read maps file");

        let maps: Vec<_> = mem_maps.lines()
            .map(|line| {
                line.parse::<MappedMemoryRegionMetadata>().expect("Unable to parse maps file")
            })
        .collect();

        Ok(maps)
    }

    fn get_map_by_name(&self, map_name: &str) -> Result<MappedMemoryRegionMetadata, ()> {
        let maps = self.get_maps().unwrap();
        let map = maps
            .iter()
            .filter(|map| map.pathname.as_ref().map(|x| x.as_ref()) == Some(map_name))
            .next()
            .unwrap()
            .clone();

        Ok(map)
    }

    fn get_memory_map(&self, map_name: &str) -> Result<Vec<u8>, ()> {
        let map = self.get_map_by_name(map_name).unwrap();

        let mem_size = map.address.end - map.address.start;
        let mut mem_file = self.mem_file.try_clone().unwrap();
        mem_file.seek(io::SeekFrom::Start(map.address.start)).unwrap();
        let mut data = vec![0u8; mem_size as usize];
        mem_file.read_exact(&mut data).unwrap();

        Ok(data)
    }

    fn stack(&self) -> Result<Vec<u8>, ()> {
        self.get_memory_map("[stack]")
    }

    fn heap(&self) -> Result<Vec<u8>, ()> {
        self.get_memory_map("[heap]")
    }
}

impl Drop for ProcessLocker {
    fn drop(&mut self) {
        let _ = unsafe { libc::kill(self.pid, libc::SIGCONT)};
    }
}


fn same(a: &[u8], b: &[u8]) -> Vec<bool> {
    a.iter().zip(b.iter())
        .map(|(a, b)| a == b)
        .collect()
}
fn increase(a: &[u8], b: &[u8]) -> Vec<bool> {
    a.iter().zip(b.iter())
        .map(|(a, b)| a > b)
        .collect()
}




fn main() {
    let pid: i32 = std::env::args().nth(1).expect("Provide pid").parse().expect("Provide pid");

    let mut last_stack: Option<Vec<u8>> = None;

    for i in 0..5
    {
        println!("{}", i);

        {
            let lock = ProcessLocker::lock(pid).unwrap();
            let stack = lock.stack().unwrap();

            let needle = 100u32;
            let needle_size = std::mem::size_of_val(&needle);

            if let Some(last) = last_stack {
                for (a, b) in stack.chunks(needle_size)
                    .zip(last.chunks(needle_size))
                    .filter(|(a, b)| a != b) {

                    println!("diff: {:?} {:?}", a, b);
                }

            }

            last_stack = Some(stack);
        }

        use std::thread::sleep;
        sleep(Duration::from_millis(2000));
    }



    //let mut mem = File::open(format!("/proc/{}/mem", pid)).unwrap();


    ////let re = regex::bytes::Regex::new(r"(?-u)(?P<cstr>[^\x00]+)\x00").unwrap();
    //let re = regex::bytes::Regex::new(r"(?-u)(?P<cstr>[\x20-\x7f]{3,})\x00").unwrap();
    //let re = regex::bytes::Regex::new(r"(?-u)(?P<cstr>New Game)").unwrap();

    //let mut total = 0;
    //for mapped_memory_region in mapped_memory_regions.maps.iter() {

    //    if mapped_memory_region.permissions.r && mapped_memory_region.pathname.as_ref().map(|x| x.as_ref()) != Some("[vvar]") {
    //        let instant = Instant::now();

    //        //println!("{:#?}", mapped_memory_region);
    //        //println!("mem: {:x}-{:x}", mapped_memory_region.address.start, mapped_memory_region.address.end);
    //        //println!("size= {}", mapped_memory_region.address.end - mapped_memory_region.address.start);

    //        let mem_size = mapped_memory_region.address.end - mapped_memory_region.address.start;
    //        total += mem_size;
    //        println!("running total={}", total);

    //        let mut mem = mem.try_clone().unwrap();
    //        mem.seek(io::SeekFrom::Start(mapped_memory_region.address.start)).unwrap();
    //        //let content: Result<Vec<_>, _>  = mem.take(mem_size).bytes().collect();
    //        //let content = content.expect("read mem failed");
    //        let mut data = vec![0u8; mem_size as usize];
    //        mem.read_exact(&mut data).unwrap();

    //        //println!("{:?}", data);
    //        //println!("content_len={}", content.len());

    //        let elapsed = instant.elapsed().as_millis() as f32 / 1000.;

    //        println!("elapsed={:?}", elapsed);
    //        println!("");

    //        let strings: Vec<_> = re.captures_iter(&data)
    //            .map(|c| String::from_utf8_lossy(c.name("cstr").unwrap().as_bytes()).to_owned())
    //            //.map(|c| c.name("cstr").unwrap().as_bytes())
    //            .collect();
    //        println!("{:?}", strings);

    //        //for x in data.windows(5)
    //        //    .filter(|x| { x == b"\x00\x01\x02\x03\x04"}) {
    //        //        println!("FOUND: {:?} in {:?}", x, mapped_memory_region.pathname);
    //        //    }

    //        //for x in data.windows(5)
    //        //    .filter(|x| { x == b"\x10\x11\x12\x13\x14"}) {
    //        //        println!("FOUND: {:?} in {:?}", x, mapped_memory_region.pathname);
    //        //    }

    //    }
    //}



}
