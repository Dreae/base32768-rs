#[macro_use]
extern crate lazy_static;
extern crate byteorder;

use byteorder::{BigEndian, ByteOrder};

use std::mem;

mod data;
mod errors;
mod bits_to_bits;

static POINT_LEN: usize = 15;

struct LookupTables {
    lookup_encode: Vec<Box<[u16]>>,
    lookup_decode: Vec<Box<[u16]>>,
}

macro_rules! gen_repertoires {
    ($n:expr, $e:ident, $d:ident) => {
        let rep_encode_len = $n.len() * (data::BLOCK_SIZE as usize);
        let rep_decode_len = calc_max_size($n);
        let mut encode_rep = Vec::with_capacity(rep_encode_len);
        let mut decode_rep = Vec::with_capacity(rep_decode_len);
        encode_rep.resize(rep_encode_len, 0u16);
        decode_rep.resize(rep_decode_len, std::u16::MAX);

        build_repertoire($n, &mut encode_rep, &mut decode_rep);
        $e.push(encode_rep.into_boxed_slice());
        $d.push(decode_rep.into_boxed_slice());
    }
}

// TODO: Generate these tables at compile time
lazy_static! {
    static ref LOOKUP_TABLES: LookupTables = {
        let mut encode_table = Vec::new();
        let mut decode_table = Vec::new();

        #[inline(always)]
        fn calc_max_size(block_start: &str) -> usize {
            let max_char = block_start.chars().max().unwrap();
            let mut b = [0u16; 1];
            max_char.encode_utf16(&mut b);

            (b[0] as usize) * (data::BLOCK_SIZE as usize)
        }

        fn build_repertoire(start_chars: &str, encode: &mut Vec<u16>, decode: &mut Vec<u16>) {
            let mut i = 0;
            for c in start_chars.chars() {
                if c.len_utf16() != 1 {
                    panic!("Got unexpected unicode len for block start character {}", c);
                }

                let mut b = [0; 1];
                c.encode_utf16(&mut b);

                let block_start_codepoint = b[0];
                let block_start_k = data::BLOCK_SIZE * i;
                for offset in 0..data::BLOCK_SIZE {
                    let code_point = block_start_codepoint + offset;
                    let k = block_start_k + offset;
                    encode[k as usize] = code_point;
                    decode[code_point as usize] = k;
                }

                i += 1;
            }
        }

        gen_repertoires!(data::BLOCK_START_0, encode_table, decode_table);
        gen_repertoires!(data::BLOCK_START_1, encode_table, decode_table);

        LookupTables{
            lookup_encode: encode_table,
            lookup_decode: decode_table,
        }
    };
}

#[inline]
fn calculate_encoded_length(byte_len: usize) -> Option<usize> {
    let bit_len = byte_len.checked_mul(8);
    let char_len = bit_len.and_then(|b| { b.checked_div(15) });
    let rem = bit_len.and_then(|b| { b.checked_rem(15) });

    rem.and_then(|r| {
        if r != 0 {
            char_len.and_then(|l| { l.checked_add(1) })
        } else {
            char_len
        }
    })
}

/// Encodes a slice of binary data into a UTF String
/// # Examples
///
/// ```
/// let data = [72u8, 101u8, 108u8, 108u8, 111u8];
/// let encoded = base32768::encode(&data).unwrap();
/// println!("Encoded message: {}", encoded);
/// ```
pub fn encode(buf: &[u8]) -> String {
    let mut output = match calculate_encoded_length(buf.len()) {
        Some(l) => {
            let mut v = Vec::<u16>::with_capacity(l);
            unsafe {
                v.set_len(l);
            }

            v
        },
        None => panic!("Integer overflow calculating size ouf output buffer"),
    };
    const LOW_15_BITS: u32 = 0x7FFF;
    const LOW_15_BITS_U16: u16 = 0x7FFF;
    
    let rem = buf.len() % 2;
    let start_of_rem = (buf.len() - rem) - 1;
    
    let mut output_index = 0;
    let mut input_index = 0;
    if start_of_rem > 3 {
        let input_buf = &buf[0..2];
        let input_num = BigEndian::read_u16(&input_buf);
        (&mut output)[0] = LOOKUP_TABLES.lookup_encode[0][((input_num >> 1) & LOW_15_BITS_U16) as usize];

        let mut input_buf = [0u8; 4];
        input_buf[1..].copy_from_slice(&buf[1..4]);
        let input_num = BigEndian::read_u32(&input_buf);
        (&mut output)[1] = LOOKUP_TABLES.lookup_encode[0][((input_num >> 2) & LOW_15_BITS) as usize];

        output_index = 2;
        input_index = 4;
        while input_index < start_of_rem {
            let input_buf = &buf[(input_index - 2)..(input_index + 2)];
            let input_num = BigEndian::read_u32(&input_buf);
            (&mut output)[output_index] = LOOKUP_TABLES.lookup_encode[0][((input_num >> ((output_index % 15) + 1)) & LOW_15_BITS) as usize];

            output_index += 1;
            input_index += 2
        }
    }

    let string = String::from_utf16(&output);
    if let Err(_) = string {
        panic!("Somehow managed to generate an invalid UTF-16 String");
    }
    
    string.unwrap()
}

/// Decodes a UTF String into a slice of binary data
/// # Examples
///
/// ```
/// let data = "䩲腻㐿";
/// let mut decoded = Vec::<u8>::new();
/// base32768::decode(&data, &mut decoded).unwrap();
/// println!("Decoded message: {}", String::from_utf8(decoded).unwrap());
/// ```
pub fn decode(in_str: &str, out_vec: &mut Vec<u8>) -> Result<(), errors::Base32768Error> {
    let mut ks = Vec::<u16>::new();
    let mut last_bytes_bits = 15;

    for (byte_offset, c) in in_str.char_indices() {
        if c.len_utf16() != 1 {
            return Err(errors::Base32768Error::new("Got invalid length for encoded character".to_owned()));
        }

        let mut b = [0; 1];
        c.encode_utf16(&mut b);

        for key in 0..LOOKUP_TABLES.lookup_decode.len() {
            if let Some(k) = LOOKUP_TABLES.lookup_decode[key].get(b[0] as usize) {
                if *k != std::u16::MAX {
                    if key != 0 {
                        if byte_offset != in_str.len() - 2 {
                            return Err(errors::Base32768Error::new("Got padding character in the middle of the stream".to_owned()));
                        } else {
                            last_bytes_bits = POINT_LEN - 8 * key;
                        }
                    }
                    ks.push(*k);
                }
            }
        }
    };
    let sized_bytes = bits_to_bits::resize_bytes_ex(ks.as_slice(), POINT_LEN, 8, last_bytes_bits);
    for idx in 0..sized_bytes.len() {
        if sized_bytes[idx].bits == 8 {
            out_vec.push(unsafe {
                mem::transmute::<u16, [u8; 2]>(sized_bytes[idx].bytes)[0]
            });
        }
    }

    Ok(())
}