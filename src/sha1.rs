enum Sha1 {
    Success,
    Null,
    InputTooLong,
    StateError,
}

const SHA1_HASH_SIZE: usize = 20;

pub struct Sha1Ctx {
    intermediate_hash: [u32; SHA1_HASH_SIZE / 4],
    low_length: u32,
    high_length: u32,
    message_block_index: u16,
    message_block: [u8; 64],
    corrupted: bool,
    computed: bool,
}

macro_rules! sha1_circular_shift {
    ($bits: expr, $word: expr) => {
        ($word << $bits) | ($word >> (32 - $bits))
    };
}

impl Sha1Ctx {
    pub fn new() -> Self {
        Self {
            low_length: 0,
            high_length: 0,
            message_block_index: 0,
            computed: false,
            corrupted: false,
            intermediate_hash: [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0],
            message_block: [0; 64],
        }
    }
    fn reset(&mut self) {
        *self = Self {
            low_length: 0,
            high_length: 0,
            message_block_index: 0,
            computed: false,
            corrupted: false,
            intermediate_hash: [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0],
            message_block: [0; 64],
        }
    }

    pub fn digest(&mut self, message: &str) -> [u8; SHA1_HASH_SIZE] {
        self.input(message);
        self.result()
    }

    fn input(&mut self, message: &str) {
        self.reset();
        let mut bytes = message.bytes();
        while let Some(byte) = bytes.next() {
            //if self.corrupted {break;};
            self.message_block[self.message_block_index as usize] = byte & 0xFF;
            self.low_length = self.low_length.wrapping_add(8);
            if self.low_length == 0 {
                self.high_length = self.high_length.wrapping_add(1);
                if self.high_length == 0 {
                    panic!("message too long");
                }
            }

            self.message_block_index += 1;
            if self.message_block_index == 64 {
                self.process_block();
            }
        }
    }

    fn result(&mut self) -> [u8; SHA1_HASH_SIZE] {
        if self.corrupted {
            panic!("I am corrupted");
        }

        if !self.computed {
            self.pad_message();

            for i in 0..64 {
                self.message_block[i] = 0;
            }

            self.low_length = 0;
            self.high_length = 0;
            self.computed = true;
        }

        let mut digest = [0 as u8; SHA1_HASH_SIZE];
        for i in 0..SHA1_HASH_SIZE {
            digest[i] = (self.intermediate_hash[i >> 2] >> 8 * (3 - (i & 0x03))) as u8;
        }
        digest
    }

    fn pad_message(&mut self) {
        if self.message_block_index > 55 {
            self.message_block[self.message_block_index as usize] = 0x80;
            self.message_block_index += 1;
            while self.message_block_index < 64 {
                self.message_block[self.message_block_index as usize] = 0;
                self.message_block_index += 1;
            }
            self.process_block();
            while self.message_block_index < 56 {
                self.message_block[self.message_block_index as usize] = 0;
                self.message_block_index += 1;
            }
        } else {
            self.message_block[self.message_block_index as usize] = 0x80;
            self.message_block_index += 1;
            while self.message_block_index < 56 {
                self.message_block[self.message_block_index as usize] = 0;
                self.message_block_index += 1;
            }
        }

        self.message_block[56] = (self.high_length >> 24) as u8;
        self.message_block[57] = (self.high_length >> 16) as u8;
        self.message_block[58] = (self.high_length >> 8) as u8;
        self.message_block[59] = self.high_length as u8;
        self.message_block[60] = (self.low_length >> 24) as u8;
        self.message_block[61] = (self.low_length >> 16) as u8;
        self.message_block[62] = (self.low_length >> 8) as u8;
        self.message_block[63] = (self.low_length) as u8;
        self.process_block();
    }

    fn process_block(&mut self) {
        let k = [0x5A827999 as u32, 0x6ED9EBA1, 0x8F1BBCDC, 0xCA62C1D6];
        let mut w = [0 as u32; 80];
        //let (A, B, C, D, E): (u32, u32, u32, u32, u32);

        for t in 0..16 {
            w[t] = (self.message_block[t * 4] as u32) << 24;
            w[t] |= (self.message_block[t * 4 + 1] as u32) << 16;
            w[t] |= (self.message_block[t * 4 + 2] as u32) << 8;
            w[t] |= self.message_block[t * 4 + 3] as u32;
        }

        for t in 16..80 {
            w[t] = sha1_circular_shift!(1, w[t - 3] ^ w[t - 8] ^ w[t - 14] ^ w[t - 16]);
        }

        let (mut A, mut B, mut C, mut D, mut E) = (
            self.intermediate_hash[0],
            self.intermediate_hash[1],
            self.intermediate_hash[2],
            self.intermediate_hash[3],
            self.intermediate_hash[4],
        );

        let mut temp = 0;

        for t in 0..20 {
            temp = sha1_circular_shift!(5, A)
                .wrapping_add(((B & C) | ((!B) & D)))
                .wrapping_add(E)
                .wrapping_add(w[t])
                .wrapping_add(k[0]);
            E = D;
            D = C;
            C = sha1_circular_shift!(30, B);
            B = A;
            A = temp;
        }

        for t in 20..40 {
            temp = sha1_circular_shift!(5, A)
                .wrapping_add((B ^ C ^ D))
                .wrapping_add(E)
                .wrapping_add(w[t])
                .wrapping_add(k[1]);
            E = D;
            D = C;
            C = sha1_circular_shift!(30, B);
            B = A;
            A = temp;
        }

        for t in 40..60 {
            temp = sha1_circular_shift!(5, A)
                .wrapping_add(((B & C) | (B & D) | (C & D)))
                .wrapping_add(E)
                .wrapping_add(w[t])
                .wrapping_add(k[2]);
            E = D;
            D = C;
            C = sha1_circular_shift!(30, B);
            B = A;
            A = temp;
        }

        for t in 60..80 {
            temp = sha1_circular_shift!(5, A)
                .wrapping_add(B ^ C ^ D)
                .wrapping_add(E)
                .wrapping_add(w[t])
                .wrapping_add(k[3]);
            E = D;
            D = C;
            C = sha1_circular_shift!(30, B);
            B = A;
            A = temp;
        }

        self.intermediate_hash[0] = self.intermediate_hash[0].wrapping_add(A);
        self.intermediate_hash[1] = self.intermediate_hash[1].wrapping_add(B);
        self.intermediate_hash[2] = self.intermediate_hash[2].wrapping_add(C);
        self.intermediate_hash[3] = self.intermediate_hash[3].wrapping_add(D);
        self.intermediate_hash[4] = self.intermediate_hash[4].wrapping_add(E);

        self.message_block_index = 0;
    }
}



#[cfg(test)]
mod tests {
    use super::Sha1Ctx;

    #[test]
    fn shash() {
        let inputs = [
            "abc",
            "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
            "a",
            "0123456701234567012345670123456701234567012345670123456701234567",
            "abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu",
            ""
        ];

        let outputs = [
            [
                0xA9u8, 0x99, 0x3E, 0x36, 0x47, 0x06, 0x81, 0x6A, 0xBA, 0x3E, 0x25, 0x71, 0x78,
                0x50, 0xC2, 0x6C, 0x9C, 0xD0, 0xD8, 0x9D,
            ],
            [
                0x84u8, 0x98, 0x3E, 0x44, 0x1C, 0x3B, 0xD2, 0x6E, 0xBA, 0xAE, 0x4A, 0xA1, 0xF9,
                0x51, 0x29, 0xE5, 0xE5, 0x46, 0x70, 0xF1,
            ],
            [
                134u8, 247, 228, 55, 250, 165, 167, 252, 225, 93, 29, 220, 185, 234, 234, 234, 55,
                118, 103, 184,
            ],
            [
                224u8, 192, 148, 232, 103, 239, 70, 195, 80, 239, 84, 167, 245, 157, 214, 11, 237,
                146, 174, 131,
            ],
            [
                0xa4u8, 0x9b, 0x24, 0x46, 0xa0, 0x2c, 0x64, 0x5b, 0xf4, 0x19, 0xf9, 0x95, 0xb6,
                0x70, 0x91, 0x25, 0x3a, 0x04, 0xa2, 0x59,
            ],
            [
                0xdau8, 0x39, 0xa3, 0xee, 0x5e, 0x6b, 0x4b, 0x0d, 0x32, 0x55, 0xbf, 0xef, 0x95,
                0x60, 0x18, 0x90, 0xaf, 0xd8, 0x07, 0x09,
            ],
        ];

        let mut ctx = Sha1Ctx::new();

        for i in 0..inputs.len() {
            assert_eq!(ctx.digest(inputs[i]), outputs[i]);
        }
    }
}
