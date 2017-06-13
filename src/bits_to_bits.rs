#[derive(Debug)]
pub struct ResizedByte {
  pub bytes: u16,
  pub bits: usize,
}

pub fn resize_bytes_ex<T>(in_buf: &[T], byte_size: usize, size: usize, last_byte_size: usize) -> Box<[ResizedByte]> where T: Into<u16> + Copy {
  let mut resized_bytes = Vec::<ResizedByte>::with_capacity(in_buf.len() * 2);
  let mut num_bits = 0;
  let mut bytes: u16 = 0;

  for idx in 0..in_buf.len() {
    let current_byte_size = if idx == in_buf.len() - 1 {
      last_byte_size
    } else {
      byte_size
    };
    
    let b = in_buf[idx];
    for i in (0..current_byte_size).rev() {
      let bit = (b.into() & (1u16 << i)) >> i;
      bytes = (bytes << 1) + bit;
      num_bits += 1;

      if num_bits == size {
        resized_bytes.push(ResizedByte {
          bytes: bytes,
          bits: num_bits,
        });

        bytes = 0;
        num_bits = 0;
      }
    }

    if idx == in_buf.len() - 1 && num_bits != 0 {
      resized_bytes.push(ResizedByte {
        bytes: bytes,
        bits: num_bits,
      });
    }
  }

  resized_bytes.into_boxed_slice()
}

#[cfg(test)]
mod test {
  #[test]
  fn resize_odd_bytes_correctly() {
    let bytes = super::resize_bytes_ex(&[4u8, 244u8, 13u8, 12u8, 92u8], 8, 15, 8);
    let correct_bytes = vec![
      super::ResizedByte {
        bytes: 634,
        bits: 15,
      },
      super::ResizedByte {
        bytes: 835,
        bits: 15,
      },
      super::ResizedByte {
        bytes: 92,
        bits: 10
      }
    ];

    assert_eq!(bytes.len(), correct_bytes.len());

    for b in 0..bytes.len() {
      assert_eq!(bytes[b].bytes, correct_bytes[b].bytes);
      assert_eq!(bytes[b].bits, correct_bytes[b].bits);
    }
  }

  #[test]
  fn resize_even_bytes_correctly() {
    let bytes = super::resize_bytes_ex(&[242u8, 67u8, 167u8, 208u8, 253u8, 91u8, 156u8, 21u8], 8, 15, 8);
    let correct_bytes = vec![
      super::ResizedByte {
        bytes: 31009,
        bits: 15,
      },
      super::ResizedByte {
        bytes: 27124,
        bits: 15,
      },
      super::ResizedByte {
        bytes: 8107,
        bits: 15
      },
      super::ResizedByte {
        bytes: 14785,
        bits: 15,
      },
      super::ResizedByte {
        bytes: 5,
        bits: 4,
      },
    ];

    assert_eq!(bytes.len(), correct_bytes.len());

    for b in 0..bytes.len() {
      assert_eq!(bytes[b].bytes, correct_bytes[b].bytes);
      assert_eq!(bytes[b].bits, correct_bytes[b].bits);
    }
  }
}