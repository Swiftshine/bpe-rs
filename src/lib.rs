pub mod bpe {
    /// Code based on 1994 Philip Gage

    use std::io::{Cursor, Seek};

    pub const DEFAULT_STACK_SIZE: usize = 30;

    /// Adapted from Philip Gage's `expand` function.
    pub fn decode(input: &[u8], stack_size: usize) -> Vec<u8> {
        let mut input = Cursor::new(input);
        
        let mut left = [0u8; 256];
        let mut right = [0u8; 256];
        let mut stack = vec![0u8; stack_size];
        let mut output = Vec::new();
    
        while input.position() < input.get_ref().len() as u64 {
            // set left to itself as literal flag
            for i in 0..256 {
                left[i] = i as u8;
            }
    
            // read pair table
            let mut c = 0;
            while c < 256 {
                let mut count = input.get_ref()[input.position() as usize] as i16;
                let _ = input.seek_relative(1);
    
                // skip range of literal bytes
                if count > 127 {
                    c += count - 127;
                    count = 0;
                }
                if c == 256 {
                    break;
                }
    
                // read pairs, skip right if literal
                for _ in 0..=count {
                    left[c as usize] = input.get_ref()[input.position() as usize];
                    let _ = input.seek_relative(1);

                    if c != left[c as usize] as i16 {
                        right[c as usize] = input.get_ref()[input.position() as usize];
                        let _ = input.seek_relative(1);
                    }
                    c += 1;
                }
                if c == 256 {
                    break;
                }
            }
            
            // calculate packed data block size
            let size = 256 * input.get_ref()[input.position() as usize] as i16 + input.get_ref()[input.position() as usize + 1] as i16;
            let _ = input.seek_relative(2);
    
            // unpack data block
            let mut i = 0;
            let mut current_size = size;

            while current_size > 0 {
                let c;
                if i > 0 {
                    i -= 1;
                    c = stack[i];
                } else {
                    current_size -= 1;
                    c = input.get_ref()[input.position() as usize];
                    let _ = input.seek_relative(1);
                }
    
                // output byte or push pair on stack
                if c == left[c as usize] {
                    output.push(c);
                } else {
                    stack[i] = right[c as usize];
                    i += 1;
                    stack[i] = left[c as usize];
                    i += 1;
                }
            }
        }
    
        output
    }

    const BLOCKSIZE: usize = 10_000;
    const HASHSIZE: usize = 8192;
    const _MAXCHARS: usize = 220;
    const _THRESHOLD: usize = 3;

    static mut ENC_BUFFER: [u8; BLOCKSIZE] = [0u8; BLOCKSIZE];
    static mut ENC_LEFTCODE: [u8; 256] = [0u8; 256];
    static mut ENC_RIGHTCODE: [u8; 256] = [0u8; 256];
    static mut ENC_COUNT: [u8; HASHSIZE] = [0u8; HASHSIZE];
    static mut ENC_LEFT: [u8; HASHSIZE] = [0u8; HASHSIZE];
    static mut ENC_RIGHT: [u8; HASHSIZE] = [0u8; HASHSIZE];
    static mut ENC_SIZE: i32 = 0;

    fn getc(file: &mut Cursor<&[u8]>) -> i32 {
        let c;
        
        if file.position() as usize >= file.get_ref().len() {
            c = -(1i32);
        } else {
            c = file.get_ref()[file.position() as usize] as i32;
            let _ = file.seek_relative(1);
        }

        c
    }


    unsafe fn lookup(a: u8, b: u8, hs: i32) -> i32 {
        let mut index;

        index = (a as i32 ^ ((b << 5) as i32)) as i32 & (hs - 1 as i32);


        while (ENC_LEFT[index as usize] as i32 != a as i32
            || ENC_RIGHT[index as usize] as i32 != b as i32)
            && ENC_COUNT[index as usize] as i32 != 0 
        {
            index = (index + 1) & (hs - 1);
        }

        ENC_LEFT[index as usize] = a;
        ENC_RIGHT[index as usize] = b;

        index
    }

    unsafe fn fileread(input: &mut Cursor<&[u8]>, bs: i32, hs: i32, mc: i32) -> bool {
        let mut index;
        let mut used = 0i32;
        
        for c in 0..hs {
            ENC_COUNT[c as usize] = 0;
        }

        for c in 0..256 {
            ENC_LEFTCODE[c as usize] = c as u8;
            ENC_RIGHTCODE[c as usize] = 0;
        }
        
        
        let mut c = 0i32;
        ENC_SIZE = 0;
        
        while ENC_SIZE < bs && used < mc
            && {
                c = getc(input);

                c != -(1 as i32)
            }
        {
            if ENC_SIZE > 0 {
                index = lookup(
                    ENC_BUFFER[(ENC_SIZE - 1 as i32) as usize],
                    c as u8,
                    hs
                );

                if (ENC_COUNT[index as usize] as i32) < 255 {
                    ENC_COUNT[index as usize] = (ENC_COUNT[index as usize]).wrapping_add(1);
                }
            }

            ENC_BUFFER[ENC_SIZE as usize] = c as u8;
            ENC_SIZE += 1;

            if ENC_RIGHTCODE[c as usize] == 0 {
                ENC_RIGHTCODE[c as usize] = 1;
                used += 1;
            }
        }


        c == -(1i32)
    }

    unsafe fn filewrite(file: &mut Vec<u8>) {
        let mut len;
        let mut c = 0i32;

        while c < 256 {
            if c == ENC_LEFTCODE[c as usize] as i32 {
                len = 1;
                c += 1;
                
                while len < 127 && c < 256
                    && c == ENC_LEFTCODE[c as usize] as i32 
                    {
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

                while len < 127 && c < 256
                    && c != ENC_LEFTCODE[c as usize] as i32
                    || len < 125 && c < 254 
                    && c + 1 != ENC_LEFTCODE[(c + 1) as usize] as i32
                {
                    len += 1;
                    c += 1;
                }

                file.push(len as u8);
                c -= len + 1;
            }

            for _ in 0..=len {
                file.push(ENC_LEFTCODE[c as usize]);

                if c != ENC_LEFTCODE[c as usize] as i32 {
                    file.push(ENC_RIGHTCODE[c as usize])
                }

                c += 1;
            }
        }

        file.push((ENC_SIZE / 256) as u8);
        file.push((ENC_SIZE % 256) as u8);

        // file.extend_from_slice(enc_buffer);
        // file.extend(enc_buffer);

        // for byte in ENC_BUFFER {
        //     file.push(byte);
        // }

        // file.extend(ENC_BUFFER[0..ENC_SIZE as usize]);

        // for i in 0..ENC_SIZE {
        //     file.push(ENC_BUFFER[i as usize]);
        // }

        file.extend_from_slice(&ENC_BUFFER[..ENC_SIZE as usize]);
    }

    /// Adapted from Philip Gage's `compress` function.
    pub fn encode(input: &[u8]) -> Vec<u8> {
        let mut input = Cursor::new(input);

        let (bs, hs, mc, th) = (8192, 4096, 200, 3);
        let mut code: i32;
        let mut leftch = 0;
        let mut rightch = 0;
        let mut output = Vec::new();
        
        // compress each data block until EOF
        unsafe {
            let mut done = false;
            while !done {
                done = fileread(&mut input, bs, hs, mc);
                code = 256;
                
                // compress this block
                loop {
                    // get next unused char for pair code
                    code -= 1;
                    
                    while code >= 0 {
                        if code == ENC_LEFTCODE[code as usize] as i32
                            && ENC_RIGHTCODE[code as usize] == 0
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
                        if ENC_COUNT[index as usize] as i32 > best {
                            best = ENC_COUNT[index as usize] as i32;
                            leftch = ENC_LEFT[index as usize] as i32;
                            rightch = ENC_RIGHT[index as usize] as i32;
                        }

                        index += 1;
                    }

                    if best < th {
                        break;
                    }

                    let oldsize = ENC_SIZE - 1 as i32;
                    let mut w = 0i32;
                    let mut r = 0i32;

                    while r < oldsize {
                        if ENC_BUFFER[r as usize] as i32 == leftch
                            && ENC_BUFFER[(r + 1 as i32) as usize] as i32 == rightch
                        {
                            if r > 0 {
                                index = lookup(
                                    ENC_BUFFER[(w - 1 as i32) as usize],
                                    leftch as u8,
                                    hs
                                );

                                if ENC_COUNT[index as usize] as i32 > 1 {
                                    ENC_COUNT[index as usize] = (ENC_COUNT[index as usize]).wrapping_sub(1);
                                }

                                index = lookup(
                                    ENC_BUFFER[(w - 1 as i32) as usize],
                                    code as u8,
                                    hs
                                );

                                if (ENC_COUNT[index as usize] as i32) < 255 {
                                    ENC_COUNT[index as usize] = (ENC_COUNT[index as usize]).wrapping_add(1);
                                }
                            }

                            if r < oldsize - 1 as i32 {
                                index = lookup(
                                    rightch as u8,
                                    ENC_BUFFER[(r + 2 as i32) as usize],
                                    hs
                                );

                                if ENC_COUNT[index as usize] as i32 > 1 {
                                    ENC_COUNT[index as usize] = (ENC_COUNT[index as usize]).wrapping_sub(1);
                                }

                                index = lookup(
                                    code as u8,
                                    ENC_BUFFER[(r + 2 as i32) as usize],
                                    hs
                                );

                                if (ENC_COUNT[index as usize] as i32) < 255 {
                                    ENC_COUNT[index as usize] = (ENC_COUNT[index as usize]).wrapping_add(1);
                                }
                            }

                            ENC_BUFFER[w as usize] = code as u8;
                            w += 1;
                            r += 1;
                            ENC_SIZE -= 1;
                        } else {
                            ENC_BUFFER[w as usize] = ENC_BUFFER[r as usize];
                            w += 1;
                        }

                        r += 1;
                    }

                    ENC_BUFFER[w as usize] = ENC_BUFFER[r as usize];
                    ENC_LEFTCODE[code as usize] = leftch as u8;
                    ENC_RIGHTCODE[code as usize] = rightch as u8;
                    index = lookup(leftch as u8, rightch as u8, hs);
                    ENC_COUNT[index as usize] = 1 as i32 as u8;
                }

                filewrite(&mut output);
            }
        }

        output
    }
    
    
}
