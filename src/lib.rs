#![feature(io)]

#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use std::error::Error;
use std::mem;

mod data;
mod errors;
mod bits_to_bits;

static POINT_LEN: usize = 15;

struct LookupTables {
    lookup_encode: HashMap<usize, HashMap<u16, u16>>,
    lookup_decode: HashMap<usize, HashMap<u16, u16>>,
}

lazy_static! {
    static ref LOOKUP_TABLES: LookupTables = {
        let mut encode = HashMap::new();
        let mut decode = HashMap::new();
        
        fn build_repertoire(start_chars: &str) -> (HashMap<u16, u16>, HashMap<u16, u16>) {
            let mut encode = HashMap::new();
            let mut decode = HashMap::new();
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
                    encode.insert(k, code_point);
                    decode.insert(code_point, k);
                }

                i += 1;
            }

            (encode, decode)
        }

        let (block_0_encode, block_0_decode) = build_repertoire(data::BLOCK_START_0);
        encode.insert(0, block_0_encode);
        decode.insert(0, block_0_decode);

        let (block_1_encode, block_1_decode) = build_repertoire(data::BLOCK_START_1);
        encode.insert(1, block_1_encode);
        decode.insert(1, block_1_decode);

        LookupTables{
            lookup_encode: encode,
            lookup_decode: decode,
        }
    };
}

/// Encodes a slice of binary data into a UTF String
/// # Examples
///
/// ```
/// let data = [72u8, 101u8, 108u8, 108u8, 111u8];
/// let encoded = base32768::encode(&data).unwrap();
/// println!("Encoded message: {}", encoded);
/// ```
pub fn encode(buf: &[u8]) -> Result<String, errors::Base32768Error> {
    let resized_bytes = bits_to_bits::resize_bytes(buf, 8, POINT_LEN);
    let mut output = Vec::<u16>::new();

    for idx in 0..resized_bytes.len() {
        let b = &resized_bytes[idx];
        let mut bytes = b.bytes;
        let mut bits = b.bits;
        if bits != POINT_LEN {
            if idx != resized_bytes.len() - 1 {
                return Err(errors::Base32768Error::new("Found partial byte midway through stream".to_owned()))
            }

            let pad_bits = (POINT_LEN - bits) % 8;
            bytes = (bytes << pad_bits) + ((1 << pad_bits) - 1);
            bits += pad_bits;
        }

        let repertoire = (POINT_LEN - bits) / 8;
        let encode_table = LOOKUP_TABLES.lookup_encode.get(&repertoire);
        if let None = encode_table {
            return Err(errors::Base32768Error::new(format!("Unrecognized `repertoire` {}", repertoire)));
        }
        let code_point = encode_table.unwrap().get(&bytes);
        if let None = code_point {
            return Err(errors::Base32768Error::new(format!("Can't encode {}", bytes)));
        }

        output.push(*code_point.unwrap());
    };

    let string = String::from_utf16(&output);
    if let Err(e) = string {
        return Err(errors::Base32768Error::new(format!("Error encoding {}", e.description())));
    }
    
    Ok(string.unwrap())
}

/// Decodes a UTF String into a slice of binary data
/// # Examples
///
/// ```
/// let data = "䩲腻㐿";
/// let res = base32768::decode(&data).unwrap();
/// println!("Encoded message: {:?}", res);
/// ```
pub fn decode(in_str: &str) -> Result<Box<[u8]>, errors::Base32768Error> {
    let mut ks = Vec::<u16>::new();
    let mut last_bytes_bits = 15;

    for (byte_offset, c) in in_str.char_indices() {
        if c.len_utf16() != 1 {
            return Err(errors::Base32768Error::new("Got invalid length for encoded character".to_owned()));
        }

        let mut b = [0; 1];
        c.encode_utf16(&mut b);

        for key in LOOKUP_TABLES.lookup_decode.keys() {
            if let Some(k) = LOOKUP_TABLES.lookup_decode.get(key).unwrap().get(&b[0]) {
                if *key != 0 {
                    if byte_offset != in_str.len() - 2 {
                        return Err(errors::Base32768Error::new("Got padding character in the middle of the stream".to_owned()));
                    } else {
                        last_bytes_bits = POINT_LEN - 8 * (*key);
                    }
                }
                ks.push(*k);
            }
        }
    };
    let sized_bytes = bits_to_bits::resize_bytes_ex(ks.as_slice(), POINT_LEN, 8, last_bytes_bits);
    let mut out_buf = Vec::<u8>::with_capacity(sized_bytes.len());
    for idx in 0..sized_bytes.len() {
        if sized_bytes[idx].bits == 8 {
            out_buf.push(unsafe {
                mem::transmute::<u16, [u8; 2]>(sized_bytes[idx].bytes)[0]
            });
        }
    }

    Ok(out_buf.into_boxed_slice())
}


#[cfg(test)]
mod test {
    extern crate glob;

    use std::fs::File;
    use std::path::Path;
    use std::io::Read;
    use std::error::Error;

    #[test]
    fn test_encode_hello() {
        let res = super::encode(&[72u8, 101u8, 108u8, 108u8, 111u8]);
        assert_eq!(res.unwrap(), "䩲腻㐿");
    }

    #[test]
    fn test_decode_hello() {
        let hello = [72u8, 101u8, 108u8, 108u8, 111u8];
        let res = super::decode("䩲腻㐿").unwrap();

        assert_eq!(*res, hello);
    }

    #[test]
    fn run_encode_decode_test_suite() {
        let src_dir = Path::new(file!()).parent().unwrap().to_str().unwrap();
        for entry in glob::glob(&format!("{}/test/**/*.bin", src_dir)).expect("Failed to glob test directory") {
            if let Ok(path) = entry {
                let path_str = path.into_os_string().into_string().unwrap();
                let mut bin_file = File::open(path_str.clone()).unwrap();
                let txt_file = File::open(path_str.replace(".bin", ".txt")).unwrap();

                let mut bin_vec = Vec::<u8>::new();

                bin_file.read_to_end(&mut bin_vec).unwrap();
                // TODO: Remove unstable feature requirement
                let test_string: String = txt_file.chars().map(|c| c.unwrap()).collect();

                let res = super::encode(&bin_vec);
                if let Err(e) = res {
                    panic!("Got error {} trying to encode from file {}", e.description(), path_str);
                }
                let out = res.unwrap();
                assert_eq!(out, test_string);

                let decoded = super::decode(&out);
                if let Err(e) = decoded {
                    panic!("Got error {} trying to decode from file {}", e.description(), path_str);
                }
                assert_eq!(&*decoded.unwrap(), bin_vec.as_slice());
            }
        }
    }
}