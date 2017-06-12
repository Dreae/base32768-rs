#![feature(test)]
extern crate test;
extern crate base32768;

use test::Bencher;

static TEST_STRING: &str = "The quick brown fox jumps over the lazy dog";

#[bench]
fn bench_base32768_encode(b: &mut Bencher) {
  let bytes = TEST_STRING.as_bytes();
  b.iter(|| { base32768::encode(&bytes) })
}