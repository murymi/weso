use std::{char, io::Bytes, mem, time::SystemTime};

const ALPHABET: [u8; 64] = [
    b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'J', b'K', b'L', b'M', b'N', b'O', b'P',
    b'Q', b'R', b'S', b'T', b'U', b'V', b'W', b'X', b'Y', b'Z', b'a', b'b', b'c', b'd', b'e', b'f',
    b'g', b'h', b'i', b'j', b'k', b'l', b'm', b'n', b'o', b'p', b'q', b'r', b's', b't', b'u', b'v',
    b'w', b'x', b'y', b'z', b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'+', b'/',
];

pub fn encode(input: &[u8]) -> String {
    let mut output = String::new();
    let l = input.len() - input.len() % 3;
    let rem = input.len() - l;
    let mut index = 0;
    let mut out_index = 0;
    loop {
        let chunk = &input[index..index+3];
        index += 3;
        let char_1 = chunk[0];
        let char_2 = chunk[1];
        let char_3 = chunk[2];
        let integer = u32::from_be_bytes([char_1, char_2, char_3, 0]);
        for i in 0..4 {
            let idx = (integer >> (26 - i * 6)) & 0x3f;
            output.push(ALPHABET[idx as usize] as char) ;
            out_index += 1;
        }
        if index == l {
            break;
        }
    }

    if rem == 2 {
        output.push(ALPHABET[(input[index] >> 2) as usize] as char);
        output.push(ALPHABET[((input[index] & 3) << 4 | (input[index+1] >> 4)) as usize] as char);
        output.push(ALPHABET[((input[index+1] & 0xf) << 2)as usize] as char);
        out_index += 3;
    } else if rem == 1 {
        output.push(ALPHABET[(input[index] >> 2) as usize] as char);
        output.push(ALPHABET[((input[index] & 3) << 4) as usize] as char);
        out_index += 2;
    }

    if rem > 0 {
        for _ in 0..(3 - rem) {
            output.push(b'=' as char);
            out_index += 1;
        } 
    }
    output
}

pub fn decode(input: &str) -> String {
    let mut lookup = [0u8; 256];
    for (index, c) in ALPHABET.iter().enumerate() {
        lookup[*c as usize] = index as u8;
    }
    let mut output = String::new();
    let mut acc:u64 = 0;
    let mut acc_len = 0;
    for (_, c) in input.bytes().enumerate() {
        if c == b'=' {
            break;
        }
        acc = acc << 6 | ((lookup[c as usize] as u64) & 63);
        acc_len += 1;
        if acc_len == 8 {
            for i in 0..6 {
                output.push(((acc >> (40 - (8 * i))) & 255) as u8 as char);
            }
            acc_len = 0;
        }
    }

    if acc_len > 0 {
        let bits = acc_len * 6;
        let bytes = bits/8;
        acc >>= bits % 8; 
        if bytes != 0 {
            for i in 0..bytes {
                output.push((acc >> (((bytes * 8) - 8) - (8 * i)) & 255) as u8 as char);
            }
        }
    }

    output
}

fn main() {
    //let mut output = [0u8; 1000];
    //let start = SystemTime::now();
    //encode("hello world".as_bytes(), &mut output);
    //let time = start.elapsed().unwrap().as_nanos();

    let d = 127;

    let message = "Zm9vYmFyZm9vYmFyZm9vYmFy";

    let output = decode(&message);

    println!("{} {}", output, output.len());
}
