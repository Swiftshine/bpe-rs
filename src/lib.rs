pub mod bpe {
    use std::io::{Cursor, Seek};

    pub const DEFAULT_STACK_SIZE: usize = 30;

    /// Adapted from Philip Gage's `expand` function.
    /// 
    /// 1994 Philip Gage
    pub fn decode(input: &[u8], stack_size: usize) -> Vec<u8> {
        let mut seeker = Cursor::new(input);
        
        let mut left = [0u8; 256];
        let mut right = [0u8; 256];
        let mut stack = vec![0u8; stack_size];
        let mut output = Vec::new();
    
        while seeker.position() < seeker.get_ref().len() as u64 {
            // set left to itself as literal flag
            for i in 0..256 {
                left[i] = i as u8;
            }
    
            // read pair table
            let mut c = 0;
            while c < 256 {
                let mut count = seeker.get_ref()[seeker.position() as usize] as i16;
                let _ = seeker.seek_relative(1);
    
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
                    left[c as usize] = seeker.get_ref()[seeker.position() as usize];
                    let _ = seeker.seek_relative(1);

                    if c != left[c as usize] as i16 {
                        right[c as usize] = seeker.get_ref()[seeker.position() as usize];
                        let _ = seeker.seek_relative(1);
                    }
                    c += 1;
                }
                if c == 256 {
                    break;
                }
            }
            
            // calculate packed data block size
            let size = 256 * seeker.get_ref()[seeker.position() as usize] as i16 + seeker.get_ref()[seeker.position() as usize + 1] as i16;
            let _ = seeker.seek_relative(2);
    
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
                    c = seeker.get_ref()[seeker.position() as usize];
                    let _ = seeker.seek_relative(1);
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

    pub fn encode(_input: &[u8]) -> Vec<u8> {
        todo!()
    }
    
}
