use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;


fn entropy(input: &[u8]) -> f32 {
    let base = 2f32;
    let size = input.len();

    let mut frequencies = HashMap::with_capacity(256);
    for b in 0..=255u8 {
        frequencies.insert(b, 0);
    }

    for b in input {
        let entry = frequencies.get_mut(b).unwrap();
        *entry += 1;
    }

    let frequencies: HashMap<_, _> = frequencies
        .iter()
        .map(|(k, v)| (k, (*v as f32) / size as f32))
        .collect();


    let mut entropy = 0f32;
    for x in frequencies.values() {
        if *x > 0f32 {
            entropy += x * x.log(base);
        }
    }


    -entropy
}


fn chunks_entropy(input: &[u8], chunksize: usize) -> Vec<(usize, f32)> {
    let mut result = Vec::new();

    for (idx, chunk) in input.chunks(chunksize).enumerate() {
        result.push((idx * chunksize, entropy(chunk)));
    }

    result
}


fn main() {
    let filename = std::env::args().nth(1).expect("Provide file");
    let chunksize: Option<usize> = std::env::args().nth(2).map(|x| x.parse().expect("Enter a number"));

    let mut f = File::open(filename).unwrap();
    let mut input = Vec::new();
    f.read_to_end(&mut input).unwrap();

    if let Some(chunksize) = chunksize {
        let en = chunks_entropy(&input, chunksize);

        for (idx, en) in &en {
            println!("{}-{}: {}", idx, idx+chunksize, en);
        }
    }
    else {
        let en = entropy(&input);
        println!("entropy = {}", en);
    }
}

