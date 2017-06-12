#![feature(io)]
extern crate glob;
extern crate base32768;

use std::fs::File;
use std::path::Path;
use std::io::Read;
use std::error::Error;

#[test]
fn test_encode_hello() {
    let res = base32768::encode(&[72u8, 101u8, 108u8, 108u8, 111u8]);
    assert_eq!(res.unwrap(), "䩲腻㐿");
}

#[test]
fn test_decode_hello() {
    let hello = [72u8, 101u8, 108u8, 108u8, 111u8];
    let mut decoded = Vec::<u8>::new();
    base32768::decode("䩲腻㐿", &mut decoded).unwrap();

    assert_eq!(decoded.as_slice(), hello);
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

            let res = base32768::encode(&bin_vec);
            if let Err(e) = res {
                panic!("Got error {} trying to encode from file {}", e.description(), path_str);
            }
            let out = res.unwrap();
            assert_eq!(out, test_string);

            let mut decoded = Vec::<u8>::new();
            let res = base32768::decode(&out, &mut decoded);
            if let Err(e) = res {
                panic!("Got error {} trying to decode from file {}", e.description(), path_str);
            }
            assert_eq!(decoded.as_slice(), bin_vec.as_slice());
        }
    }
}