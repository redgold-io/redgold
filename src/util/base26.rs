use bdk::bitcoin::util::base58::Error;
use std::{fmt, iter, slice, str};
use bdk::bitcoin::util::base58;

static BASE26_CHARS: &'static [u8] = b"abcdefghijklmnopqrstuvwxyz";
// use num_traits::cast::ToPrimitive;

static BASE26_DIGITS: [Option<u8>; 128] = [
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None, // 0-7
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None, // 8-15
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None, // 16-23
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None, // 24-31
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None, // 32-39
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None, // 40-47
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None, // 48-55
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None, // 56-63
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None, // 64-71
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None, // 72-79
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None, // 80-87
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None, // 88-95
    None,
    Some(0),
    Some(1),
    Some(2),
    Some(3),
    Some(4),
    Some(5),
    Some(6), // 96-103
    Some(7),
    Some(8),
    Some(9),
    Some(10),
    Some(11),
    Some(12),
    Some(13),
    Some(14), // 104-111
    Some(15),
    Some(16),
    Some(17),
    Some(18),
    Some(19),
    Some(20),
    Some(21),
    Some(22), // 112-119
    Some(23),
    Some(24),
    Some(25),
    None,
    None,
    None,
    None,
    None, // 120-127
];

/// Decode base26-encoded string into a byte vector
pub fn from(data: &str) -> Result<Vec<u8>, Error> {
    // 11/15 is just over log_256(58)
    let mut scratch = vec![0u8; 1 + data.len() * 9 / 15];
    // Build in base 256
    for d58 in data.bytes() {
        // Compute "X = X * 58 + next_digit" in base 256
        if d58 as usize > BASE26_DIGITS.len() {
            return Err(Error::BadByte(d58));
        }
        let mut carry = match BASE26_DIGITS[d58 as usize] {
            Some(d58) => d58 as u32,
            None => {
                return Err(Error::BadByte(d58));
            }
        };
        for d256 in scratch.iter_mut().rev() {
            carry += *d256 as u32 * 26;
            *d256 = carry as u8;
            carry /= 256;
        }
        assert_eq!(carry, 0);
    }

    // Copy leading zeroes directly
    let mut ret: Vec<u8> = data
        .bytes()
        .take_while(|&x| x == BASE26_CHARS[0])
        .map(|_| 0)
        .collect();
    // Copy rest of string
    ret.extend(scratch.into_iter().skip_while(|&x| x == 0));
    Ok(ret)
}

impl<T: Default + Copy> SmallVec<T> {
    pub fn new() -> SmallVec<T> {
        SmallVec {
            len: 0,
            stack: [T::default(); 100],
            heap: Vec::new(),
        }
    }

    pub fn push(&mut self, val: T) {
        if self.len < 100 {
            self.stack[self.len] = val;
            self.len += 1;
        } else {
            self.heap.push(val);
        }
    }

    pub fn iter(&self) -> iter::Chain<slice::Iter<T>, slice::Iter<T>> {
        // If len<100 then we just append an empty vec
        self.stack[0..self.len].iter().chain(self.heap.iter())
    }

    pub fn iter_mut(&mut self) -> iter::Chain<slice::IterMut<T>, slice::IterMut<T>> {
        // If len<100 then we just append an empty vec
        self.stack[0..self.len]
            .iter_mut()
            .chain(self.heap.iter_mut())
    }
}

/// Vector-like object that holds the first 100 elements on the stack. If more space is needed it
/// will be allocated on the heap.
struct SmallVec<T> {
    len: usize,
    stack: [T; 100],
    heap: Vec<T>,
}

fn format_iter<I, W>(writer: &mut W, data: I) -> Result<(), fmt::Error>
where
    I: Iterator<Item = u8> + Clone,
    W: fmt::Write,
{
    let mut ret = SmallVec::new();

    let mut leading_zero_count = 0;
    let mut leading_zeroes = true;
    // Build string in little endian with 0-58 in place of characters...
    for d256 in data {
        let mut carry = d256 as usize;
        if leading_zeroes && carry == 0 {
            leading_zero_count += 1;
        } else {
            leading_zeroes = false;
        }

        for ch in ret.iter_mut() {
            let new_ch = *ch as usize * 256 + carry;
            *ch = (new_ch % 26) as u8;
            carry = new_ch / 26;
        }
        while carry > 0 {
            ret.push((carry % 26) as u8);
            carry /= 26;
        }
    }

    // ... then reverse it and convert to chars
    for _ in 0..leading_zero_count {
        ret.push(0);
    }

    for ch in ret.iter().rev() {
        writer.write_char(BASE26_CHARS[*ch as usize] as char)?;
    }

    Ok(())
}

fn encode_iter<I>(data: I) -> String
where
    I: Iterator<Item = u8> + Clone,
{
    let mut ret = String::new();
    format_iter(&mut ret, data).expect("writing into string shouldn't fail");
    ret
}

/// Directly encode a slice as base58
pub fn encode_slice(data: &[u8]) -> String {
    encode_iter(data.iter().cloned())
}

#[test]
fn test() {
    let data = crate::util::sha256_str("asdf").to_vec();
    println!("{}", hex::encode(data.clone()));
    println!("{}", base58::check_encode_slice(&data));
    let res = encode_slice(&data.clone());
    println!("{}", res);
    println!("{}", res.to_uppercase());
    let res2 = from(&res.clone()).expect("decoded");
    // println!("{}", hex::encode(res2.clone()));
    assert_eq!(data, res2.clone());

    let res3 = encode_slice(&res2.clone());
    assert_eq!(res3, res);
    // for c in BASE26_CHARS {
    //     println!("{:?}, {:?}", c, c)
    // }
}
