pub mod bpe {
    /// Code based on 1994 Philip Gage
    use std::io::{Cursor, Seek};

    /// The recommended "stack" size for decoding.
    pub const DEFAULT_STACK_SIZE: usize = 5000;

    const EOF: i32 = -1;

    fn getc(file: &mut Cursor<&[u8]>) -> i32 {
        let c;

        if file.position() as usize >= file.get_ref().len() {
            c = EOF;
        } else {
            c = file.get_ref()[file.position() as usize] as i32;
            let _ = file.seek_relative(1);
        }

        c
    }

    /// Adapted from Philip Gage's `expand` function.
    ///
    /// ### Parameters
    /// `input`: The data to be decoded.
    /// `stack_size`: The size of the "stack" for decoding.
    ///
    /// ### Returns
    /// A `Vec<u8>` of the decoded data.
    pub fn decode(input: &[u8], stack_size: usize) -> Vec<u8> {
        let mut input = Cursor::new(input);
        let mut output = Vec::new();

        let mut left = [0u8; 256];
        let mut right = [0u8; 256];
        let mut stack = vec![0u8; stack_size];

        let mut count: u16;
        let mut c: u16;

        loop {
            count = getc(&mut input) as u16;

            if count == EOF as u16 {
                break;
            }

            for i in 0..256 {
                left[i as usize] = i as u8;
            }

            c = 0u16;
            loop {
                if count > 127 {
                    c += count - 127;
                    count = 0;
                }

                if c == 256 {
                    break;
                }

                for _ in 0..=count {
                    left[c as usize] = getc(&mut input) as u8;
                    if c != left[c as usize] as u16 {
                        right[c as usize] = getc(&mut input) as u8;
                    }
                    c += 1;
                }

                if c == 256 {
                    break;
                }

                count = getc(&mut input) as u16;
            }

            let mut size = 256 * getc(&mut input) + getc(&mut input);

            let mut i = 0;

            loop {
                if i != 0 {
                    i -= 1;
                    c = stack[i as usize] as u16;
                } else {
                    let temp = size;
                    size -= 1;

                    if temp == 0 {
                        break;
                    }

                    c = getc(&mut input) as u16;
                }

                if c == left[c as usize] as u16 {
                    output.push(c as u8);
                } else {
                    let temp = i;
                    i += 1;
                    stack[temp as usize] = right[c as usize];

                    let temp = i;
                    i += 1;
                    stack[temp as usize] = left[c as usize];
                }
            }
        }

        output
    }
    
    const BLOCKSIZE: usize = 131072;
    const HASHSIZE: usize = 65536;
    const MAXCHARS: usize = 200;
    const THRESHOLD: usize = 3;

    struct Encoder {
        buffer: [u8; BLOCKSIZE],
        leftcode: [u8; 256],
        rightcode: [u8; 256],
        count: [u8; HASHSIZE],
        left: [u8; HASHSIZE],
        right: [u8; HASHSIZE],
        size: i32,
    }

    impl Encoder {
        fn new() -> Self {
            Self {
                buffer: [0; BLOCKSIZE],
                leftcode: [0; 256],
                rightcode: [0; 256],
                count: [0; HASHSIZE],
                left: [0; HASHSIZE],
                right: [0; HASHSIZE],
                size: 0,
            }
        }

        unsafe fn lookup(&mut self, a: u8, b: u8, hs: i32) -> i32 {
            let mut index;

            index = (a as i32 ^ (b as i32) << 5) as i32 & (hs - 1 as i32);

            while (self.left[index as usize] as i32 != a as i32
                || self.right[index as usize] as i32 != b as i32)
                && self.count[index as usize] as i32 != 0
            {
                index = (index + 1) & (hs - 1);
            }

            self.left[index as usize] = a;
            self.right[index as usize] = b;

            index
        }

        unsafe fn fileread(
            &mut self,
            input: &mut Cursor<&[u8]>,
            bs: i32,
            hs: i32,
            mc: i32,
        ) -> bool {
            let mut index;
            let mut used = 0i32;

            for c in 0..hs {
                self.count[c as usize] = 0;
            }

            for c in 0..256 {
                self.leftcode[c as usize] = c as u8;
                self.rightcode[c as usize] = 0;
            }

            let mut c = 0i32;
            self.size = 0;

            while self.size < bs && used < mc && {
                c = getc(input);
                c != EOF
            } {
                if self.size > 0 {
                    index = self.lookup(self.buffer[(self.size - 1 as i32) as usize], c as u8, hs);

                    if (self.count[index as usize] as i32) < 255 {
                        self.count[index as usize] = (self.count[index as usize]).wrapping_add(1);
                    }
                }

                self.buffer[self.size as usize] = c as u8;
                self.size += 1;

                if self.rightcode[c as usize] == 0 {
                    self.rightcode[c as usize] = 1;
                    used += 1;
                }
            }

            c == EOF
        }

        unsafe fn filewrite(&self, file: &mut Vec<u8>) {
            let mut len;
            let mut c = 0i32;
    
            while c < 256 {
                if c == self.leftcode[c as usize] as i32 {
                    len = 1;
                    c += 1;
    
                    while len < 127 && c < 256 && c == self.leftcode[c as usize] as i32 {
                        len += 1;
                        c += 1;
                    }
    
                    file.push((len + 127) as u8);
                    len = 0;
    
                    if c == 256 {
                        break;
                    }
                } else {
                    len = 0;
                    c += 1;
    
                    while (len < 127 && c < 256 && c != self.leftcode[c as usize] as i32)
                        || (len < 125 && c < 254 && c + 1 != self.leftcode[(c + 1) as usize] as i32)
                    {
                        len += 1;
                        c += 1;
                    }
    
                    file.push(len as u8);
                    c -= len + 1;
                }
    
                for _ in 0..=len {
                    file.push(self.leftcode[c as usize]);
    
                    if c != self.leftcode[c as usize] as i32 {
                        file.push(self.rightcode[c as usize])
                    }
    
                    c += 1;
                }
            }
            
            file.push((self.size / 256) as u8);
            file.push((self.size % 256) as u8);
            
            // println!("Writing bytes at offset 0x{:X}", file.len());
            file.extend_from_slice(&self.buffer[..self.size as usize]);
        }
    }


    /// Adapted from Philip Gage's `compress` function.
    ///
    /// ### Parameters
    /// `input`: The data to be encoded.
    ///
    /// ### Returns
    /// A `Vec<u8>` of the encoded data.
    pub fn encode(input: &[u8]) -> Vec<u8> {
        let mut encoder = Box::new(Encoder::new());
        let mut output: Vec<u8> = Vec::with_capacity(input.len());
        
        let mut input = Cursor::new(input);

        let (bs, hs, mc, th) = (BLOCKSIZE as i32, HASHSIZE as i32, MAXCHARS as i32, THRESHOLD as i32);
        let mut code: i32;
        let mut leftch = 0;
        let mut rightch = 0;

        // compress each data block until EOF
        unsafe {
            let mut done = false;
            while !done {
                done = encoder.fileread(&mut input, bs, hs, mc);
                code = 256;

                // compress this block
                loop {
                    // get next unused char for pair code
                    code -= 1;

                    while code >= 0 {
                        if code == encoder.leftcode[code as usize] as i32
                            && encoder.rightcode[code as usize] == 0
                        {
                            break;
                        }

                        code -= 1;
                    }

                    if code < 0 {
                        break;
                    }

                    let mut best = 2;
                    let mut index = 0;

                    while index < hs {
                        if encoder.count[index as usize] as i32 > best {
                            best = encoder.count[index as usize] as i32;
                            leftch = encoder.left[index as usize] as i32;
                            rightch = encoder.right[index as usize] as i32;
                        }

                        index += 1;
                    }

                    if best < th {
                        break;
                    }

                    let oldsize = encoder.size - 1;
                    let mut w = 0i32;
                    let mut r = 0i32;

                    while r < oldsize {
                        if encoder.buffer[r as usize] as i32 == leftch
                            && encoder.buffer[(r + 1 as i32) as usize] as i32 == rightch
                        {
                            if r > 0 {
                                index = encoder.lookup(encoder.buffer[(w - 1) as usize], leftch as u8, hs);

                                if encoder.count[index as usize] as i32 > 1 {
                                    encoder.count[index as usize] =
                                        (encoder.count[index as usize]).wrapping_sub(1);
                                }

                                index = encoder.lookup(encoder.buffer[(w - 1) as usize], code as u8, hs);

                                if (encoder.count[index as usize] as i32) < 255 {
                                    encoder.count[index as usize] =
                                        (encoder.count[index as usize]).wrapping_add(1);
                                }
                            }

                            if r < oldsize - 1 {
                                index = encoder.lookup(rightch as u8, encoder.buffer[(r + 2) as usize], hs);

                                if encoder.count[index as usize] as i32 > 1 {
                                    encoder.count[index as usize] =
                                        (encoder.count[index as usize]).wrapping_sub(1);
                                }

                                index = encoder.lookup(code as u8, encoder.buffer[(r + 2) as usize], hs);

                                if (encoder.count[index as usize] as i32) < 255 {
                                    encoder.count[index as usize] =
                                        (encoder.count[index as usize]).wrapping_add(1);
                                }
                            }

                            encoder.buffer[w as usize] = code as u8;
                            w += 1;
                            r += 1;
                            encoder.size -= 1;
                        } else {
                            encoder.buffer[w as usize] = encoder.buffer[r as usize];
                            w += 1;
                        }

                        r += 1;
                    }

                    encoder.buffer[w as usize] = encoder.buffer[r as usize];
                    encoder.leftcode[code as usize] = leftch as u8;
                    encoder.rightcode[code as usize] = rightch as u8;
                    index = encoder.lookup(leftch as u8, rightch as u8, hs);
                    encoder.count[index as usize] = 1 as u8;
                }

                encoder.filewrite(&mut output);
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn check_decode1() {
        let target = fs::read("test_files/ferris.png").unwrap();
        let source = fs::read("test_files/ferris-encoded.bin").unwrap();

        assert_eq!(target, bpe::decode(&source, bpe::DEFAULT_STACK_SIZE));
    }

    #[test]
    fn check_decode2() {
        let target = fs::read("test_files/picture.jpg").unwrap();
        let source = fs::read("test_files/picture-encoded.bin").unwrap();

        assert_eq!(target, bpe::decode(&source, bpe::DEFAULT_STACK_SIZE));
    }
}
