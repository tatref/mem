use std::io;


fn main() {
    let pid = std::process::id();

    println!("{}", pid);

    let stack_allocated_array: [u8; 5] = *b"\x00\x01\x02\x03\x04";
    let heap_allocated_array: Box<[u8; 5]> = Box::new(*b"\x10\x11\x12\x13\x14");

    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer).unwrap();

    println!("end");
    println!("{:?}", stack_allocated_array);
    println!("{:?}", heap_allocated_array);

}
