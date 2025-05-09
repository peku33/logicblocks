use anyhow::{Error, ensure};
use array_init::array_init;
use arrayvec::ArrayVec;
use itertools::Itertools;
use std::iter;

pub fn bits_array_to_byte(bits: [bool; 8]) -> u8 {
    bits.into_iter()
        .enumerate()
        .filter_map(|(index, bit)| if bit { Some(index) } else { None })
        .fold(0, |accumulator, index| accumulator | (1 << index))
}
pub fn bits_byte_to_array(bits: u8) -> [bool; 8] {
    array_init::<_, bool, 8>(|index| bits & (1 << index) != 0)
}
pub fn bits_byte_to_array_checked(
    bits: u8,
    max: usize,
) -> Result<[bool; 8], Error> {
    let bits_array = bits_byte_to_array(bits);
    ensure!(&bits_array[max..].iter().all(|bit| !bit), "bit overflow");
    Ok(bits_array)
}

pub fn bits_slice_to_bytes(bits: &[bool]) -> Box<[u8]> {
    bits.iter()
        .copied()
        .chain(iter::repeat(false))
        .take(bits.len().div_ceil(8) * 8)
        .chunks(8)
        .into_iter()
        .map(|bits_array_iterator| {
            bits_array_to_byte(
                bits_array_iterator
                    .collect::<ArrayVec<_, 8>>()
                    .into_inner()
                    .unwrap(),
            )
        })
        .collect::<Box<[_]>>()
}
pub fn bits_bytes_to_slice(bits: &[u8]) -> Box<[bool]> {
    bits.iter()
        .copied()
        .flat_map(bits_byte_to_array)
        .collect::<Box<[_]>>()
}
pub fn bits_bytes_to_slice_checked(
    bits: &[u8],
    size: usize,
) -> Result<Box<[bool]>, Error> {
    let mut bits_slice = bits_bytes_to_slice(bits).into_vec();
    ensure!(bits_slice.len() >= size, "bits underflow");
    ensure!(&bits_slice[size..].iter().all(|bit| !bit), "bits overflow");
    bits_slice.truncate(size);
    let bits_slice = bits_slice.into_boxed_slice();
    Ok(bits_slice)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_array_to_byte_1() {
        assert_eq!(
            bits_array_to_byte([true, false, true, true, false, false, true, true]),
            0xcd
        );
    }

    #[test]
    fn bits_byte_to_array_1() {
        assert_eq!(
            bits_byte_to_array(0xcd),
            [true, false, true, true, false, false, true, true]
        );
    }

    #[test]
    fn bits_byte_to_array_checked_1() {
        assert_eq!(
            bits_byte_to_array_checked(0x05, 3).unwrap(),
            [true, false, true, false, false, false, false, false]
        );
        assert!(bits_byte_to_array_checked(0x05, 2).is_err());
    }

    #[test]
    fn bits_slice_to_bytes_1() {
        assert_eq!(
            bits_slice_to_bytes(&[
                true, false, true, true, false, false, true, true, // 0xcd
                true, true, false, true, false, true, true, false, // 0x6b
                true, false, true, // 0x05
            ]),
            vec![0xcd, 0x6b, 0x05].into_boxed_slice()
        );
    }

    #[test]
    fn bits_bytes_to_slice_1() {
        assert_eq!(
            bits_bytes_to_slice(&[0xcd, 0x6b, 0x05]),
            vec![
                true, false, true, true, false, false, true, true, // 0xcd
                true, true, false, true, false, true, true, false, // 0x6b
                true, false, true, false, false, false, false, false, // 0x05
            ]
            .into_boxed_slice(),
        );
    }

    #[test]
    fn bits_bytes_to_slice_checked_1() {
        assert_eq!(
            bits_bytes_to_slice_checked(&[0xcd, 0x6b, 0x05], 19).unwrap(),
            vec![
                true, false, true, true, false, false, true, true, // 0xcd
                true, true, false, true, false, true, true, false, // 0x6b
                true, false, true, // 0x05
            ]
            .into_boxed_slice(),
        );
        assert!(bits_bytes_to_slice_checked(&[0xcd, 0x6b, 0x05], 18).is_err());
    }
}
