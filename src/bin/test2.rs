use std::io;
use std::thread::sleep;
use std::time::Duration;


fn main() {
    let pid = std::process::id();

    println!("pid = {}", pid);

    let stack_allocated_array: [u8; 5] = *b"\x00\x01\x02\x03\x04";
    let heap_allocated_array: Box<[u8; 5]> = Box::new(*b"\x10\x11\x12\x13\x14");

    for i in 0u32.. {
        sleep(Duration::from_millis(2_000));
        println!("{}", i);
    }

    println!("end");
    println!("{:?}", stack_allocated_array);
    println!("{:?}", heap_allocated_array);

}
