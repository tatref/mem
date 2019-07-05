use std::time::Duration;

use std::str::FromStr;
use std::fs::File;
use std::io::BufReader;
use std::io::{self,prelude::*};
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

pub struct ProcessLocker {
    pid: i32,
    mem_file: File,
}

impl ProcessLocker {
    pub fn lock(pid: i32) -> Result<Self, ()> {
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

    pub fn stack(&self) -> Result<Vec<u8>, ()> {
        self.get_memory_map("[stack]")
    }

    pub fn heap(&self) -> Result<Vec<u8>, ()> {
        self.get_memory_map("[heap]")
    }
}

impl Drop for ProcessLocker {
    fn drop(&mut self) {
        let _ = unsafe { libc::kill(self.pid, libc::SIGCONT)};
    }
}
