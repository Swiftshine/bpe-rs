pub mod bpe {
    /// Code based on 1994 Philip Gage
    use std::io::{Cursor, Seek};
    use std::ptr;

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
    const THRESHOLD: u16 = 3;

    #[inline(always)]
    fn hash(a: u8, b: u8) -> usize {
        ((a as usize) ^ ((b as usize) << 5)) & (HASHSIZE - 1)
    }

    struct Reader<'a> {
        data: &'a [u8],
        pos: usize,
    }

    impl<'a> Reader<'a> {
        #[inline(always)]
        fn new(data: &'a [u8]) -> Self {
            Self { data, pos: 0 }
        }

        #[inline(always)]
        fn getc(&mut self) -> Option<u8> {
            if self.pos >= self.data.len() {
                None
            } else {
                let b = unsafe { *self.data.get_unchecked(self.pos) };
                self.pos += 1;
                Some(b)
            }
        }
    }

    struct Encoder {
        buffer: Box<[u8; BLOCKSIZE]>,

        leftcode: [u8; 256],
        rightcode: [u8; 256],

        left: Box<[u8; HASHSIZE]>,
        right: Box<[u8; HASHSIZE]>,
        count: Box<[u16; HASHSIZE]>,

        size: usize,
    }

    impl Encoder {
        fn new() -> Self {
            Self {
                buffer: Box::new([0; BLOCKSIZE]),

                leftcode: [0; 256],
                rightcode: [0; 256],

                left: Box::new([0; HASHSIZE]),
                right: Box::new([0; HASHSIZE]),
                count: Box::new([0; HASHSIZE]),

                size: 0,
            }
        }

        #[inline(always)]
        unsafe fn lookup(&mut self, a: u8, b: u8) -> usize {
            let left = self.left.as_mut_ptr();
            let right = self.right.as_mut_ptr();
            let count = self.count.as_mut_ptr();

            let mut index = hash(a, b);

            while (*left.add(index) != a || *right.add(index) != b) && *count.add(index) != 0 {
                index = (index + 1) & (HASHSIZE - 1);
            }

            *left.add(index) = a;
            *right.add(index) = b;

            index
        }

        #[inline(always)]
        unsafe fn reset_tables(&mut self) {
            ptr::write_bytes(self.count.as_mut_ptr(), 0, HASHSIZE);

            for i in 0..256 {
                self.leftcode[i] = i as u8;
                self.rightcode[i] = 0;
            }

            self.size = 0;
        }

        unsafe fn fileread(&mut self, input: &mut Reader) -> bool {
            self.reset_tables();

            let buffer = self.buffer.as_mut_ptr();
            let count = self.count.as_mut_ptr();

            let mut used = 0usize;

            while self.size < BLOCKSIZE && used < MAXCHARS {
                let c = match input.getc() {
                    Some(v) => v,
                    None => return true,
                };

                if self.size > 0 {
                    let prev = *buffer.add(self.size - 1);

                    let index = self.lookup(prev, c);

                    let cnt = count.add(index);

                    if *cnt < 255 {
                        *cnt += 1;
                    }
                }

                *buffer.add(self.size) = c;
                self.size += 1;

                if self.rightcode[c as usize] == 0 {
                    self.rightcode[c as usize] = 1;
                    used += 1;
                }
            }

            false
        }

        unsafe fn filewrite(&self, output: &mut Vec<u8>) {
            let mut c = 0usize;

            while c < 256 {
                let mut len;

                if c == self.leftcode[c] as usize {
                    len = 1usize;
                    c += 1;

                    while len < 127 && c < 256 && c == self.leftcode[c] as usize {
                        len += 1;
                        c += 1;
                    }

                    output.push((len + 127) as u8);

                    len = 0;

                    if c == 256 {
                        break;
                    }
                } else {
                    len = 0usize;
                    c += 1;

                    while (len < 127 && c < 256 && c != self.leftcode[c] as usize)
                        || (len < 125 && c < 254 && c + 1 != self.leftcode[c + 1] as usize)
                    {
                        len += 1;
                        c += 1;
                    }

                    output.push(len as u8);

                    c -= len + 1;
                }

                for _ in 0..=len {
                    output.push(self.leftcode[c]);

                    if c != self.leftcode[c] as usize {
                        output.push(self.rightcode[c]);
                    }

                    c += 1;
                }
            }

            output.push((self.size >> 8) as u8);
            output.push((self.size & 0xFF) as u8);
            output.extend_from_slice(&self.buffer[..self.size]);
        }
    }

    pub fn encode(input: &[u8]) -> Vec<u8> {
        let mut encoder = Encoder::new();

        let mut output = Vec::with_capacity(input.len());

        let mut reader = Reader::new(input);

        unsafe {
            let mut done = false;

            while !done {
                done = encoder.fileread(&mut reader);

                let mut code: i32 = 256;

                loop {
                    code -= 1;

                    while code >= 0 {
                        let c = code as usize;

                        if c == encoder.leftcode[c] as usize && encoder.rightcode[c] == 0 {
                            break;
                        }

                        code -= 1;
                    }

                    if code < 0 {
                        break;
                    }

                    let mut best: u16 = 2;
                    let mut leftch = 0u8;
                    let mut rightch = 0u8;

                    {
                        let count = encoder.count.as_ptr();
                        let left = encoder.left.as_ptr();
                        let right = encoder.right.as_ptr();

                        for i in 0..HASHSIZE {
                            let cnt = *count.add(i);

                            if cnt > best {
                                best = cnt;
                                leftch = *left.add(i);
                                rightch = *right.add(i);
                            }
                        }
                    }

                    if best < THRESHOLD {
                        break;
                    }

                    let code_u8 = code as u8;

                    let buffer = encoder.buffer.as_mut_ptr();
                    let count = encoder.count.as_mut_ptr();

                    let oldsize = encoder.size - 1;

                    let mut w = 0usize;
                    let mut r = 0usize;

                    while r < oldsize {
                        if *buffer.add(r) == leftch && *buffer.add(r + 1) == rightch {
                            if r > 0 {
                                let prev = *buffer.add(w - 1);

                                let idx1 = encoder.lookup(prev, leftch);

                                if *count.add(idx1) > 1 {
                                    *count.add(idx1) -= 1;
                                }

                                let idx2 = encoder.lookup(prev, code_u8);

                                if *count.add(idx2) < 255 {
                                    *count.add(idx2) += 1;
                                }
                            }

                            if r < oldsize - 1 {
                                let next = *buffer.add(r + 2);

                                let idx1 = encoder.lookup(rightch, next);

                                if *count.add(idx1) > 1 {
                                    *count.add(idx1) -= 1;
                                }

                                let idx2 = encoder.lookup(code_u8, next);

                                if *count.add(idx2) < 255 {
                                    *count.add(idx2) += 1;
                                }
                            }

                            *buffer.add(w) = code_u8;

                            w += 1;
                            r += 2;

                            encoder.size -= 1;
                        } else {
                            *buffer.add(w) = *buffer.add(r);

                            w += 1;
                            r += 1;
                        }
                    }

                    *buffer.add(w) = *buffer.add(r);

                    encoder.leftcode[code as usize] = leftch;
                    encoder.rightcode[code as usize] = rightch;

                    let idx = encoder.lookup(leftch, rightch);

                    *count.add(idx) = 1;
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
