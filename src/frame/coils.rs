// SPDX-FileCopyrightText: Copyright (c) 2018-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::*;
use crate::error::*;

/// Packed coils
#[cfg_attr(all(feature = "defmt", target_os = "none"), derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Coils<'c> {
    pub(crate) data: RawData<'c>,
    pub(crate) quantity: usize,
}

impl<'c> Coils<'c> {
    /// Pack coils defined by an bool slice into a byte buffer.
    pub fn from_bools(bools: &[bool], target: &'c mut [u8]) -> Result<Self, Error> {
        if bools.is_empty() {
            return Err(Error::BufferSize);
        }
        pack_coils(bools, target)?;
        Ok(Coils {
            data: target,
            quantity: bools.len(),
        })
    }

    //TODO: add tests
    pub(crate) fn copy_to(&self, buf: &mut [u8]) {
        let packed_len = self.packed_len();
        debug_assert!(buf.len() >= packed_len);
        (0..packed_len).for_each(|idx| {
            buf[idx] = self.data[idx];
        });
    }

    /// Quantity of coils
    #[must_use]
    pub const fn len(&self) -> usize {
        self.quantity
    }

    /// Number of bytes required to pack the coils.
    #[must_use]
    pub const fn packed_len(&self) -> usize {
        packed_coils_len(self.quantity)
    }

    ///  Returns `true` if the container has no items.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.quantity == 0
    }

    /// Get a specific coil.
    #[must_use]
    pub const fn get(&self, idx: usize) -> Option<Coil> {
        if idx + 1 > self.quantity {
            return None;
        }
        Some((self.data[(idx as u16 / 8u16) as usize] >> (idx % 8)) & 0b1 > 0)
    }
}

/// Coils iterator.
// TODO: crate an generic iterator
#[cfg_attr(all(feature = "defmt", target_os = "none"), derive(defmt::Format))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoilsIter<'c> {
    cnt: usize,
    coils: Coils<'c>,
}

impl Iterator for CoilsIter<'_> {
    type Item = Coil;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.coils.get(self.cnt);
        self.cnt += 1;
        result
    }
}

impl<'c> IntoIterator for Coils<'c> {
    type Item = Coil;
    type IntoIter = CoilsIter<'c>;

    fn into_iter(self) -> Self::IntoIter {
        CoilsIter {
            cnt: 0,
            coils: self,
        }
    }
}

/// Turn a bool into a u16 coil value
#[must_use]
pub const fn bool_to_u16_coil(state: bool) -> u16 {
    if state { 0xFF00 } else { 0x0000 }
}

/// Turn a u16 coil value into a boolean value.
pub const fn u16_coil_to_bool(coil: u16) -> Result<bool, Error> {
    match coil {
        0xFF00 => Ok(true),
        0x0000 => Ok(false),
        _ => Err(Error::CoilValue(coil)),
    }
}

/// Calculate the number of bytes required for a given number of coils.
#[must_use]
pub const fn packed_coils_len(bitcount: usize) -> usize {
    bitcount.div_ceil(8)
}

///  Pack coils into a byte array.
///
///  It returns the number of bytes used to pack the coils.
pub fn pack_coils(coils: &[Coil], bytes: &mut [u8]) -> Result<usize, Error> {
    let packed_size = packed_coils_len(coils.len());
    if bytes.len() < packed_size {
        return Err(Error::BufferSize);
    }
    coils.iter().enumerate().for_each(|(i, b)| {
        let v = u8::from(*b);
        bytes[i / 8] |= v << (i % 8);
    });
    Ok(packed_size)
}

///  Unpack coils from a byte array.
pub fn unpack_coils(bytes: &[u8], count: u16, coils: &mut [Coil]) -> Result<(), Error> {
    if coils.len() < count as usize {
        return Err(Error::BufferSize);
    }
    (0..count).for_each(|i| {
        coils[i as usize] = (bytes[(i / 8u16) as usize] >> (i % 8)) & 0b1 > 0;
    });
    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn from_bool_slice() {
        let bools: &[bool] = &[true, false, true, true];
        let buff: &mut [u8] = &mut [0];
        let coils = Coils::from_bools(bools, buff).unwrap();
        assert_eq!(coils.len(), 4);
        let mut iter = coils.into_iter();
        assert_eq!(iter.next(), Some(true));
        assert_eq!(iter.next(), Some(false));
        assert_eq!(iter.next(), Some(true));
        assert_eq!(iter.next(), Some(true));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn coils_len() {
        let coils = Coils {
            data: &[0, 1, 2],
            quantity: 5,
        };
        assert_eq!(coils.len(), 5);
    }

    #[test]
    fn coils_empty() {
        let coils = Coils {
            data: &[0, 1, 2],
            quantity: 0,
        };
        assert!(coils.is_empty());
    }

    #[test]
    fn coils_get() {
        let coils = Coils {
            data: &[0b1],
            quantity: 1,
        };
        assert_eq!(coils.get(0), Some(true));
        assert_eq!(coils.get(1), None);

        let coils = Coils {
            data: &[0b01],
            quantity: 2,
        };
        assert_eq!(coils.get(0), Some(true));
        assert_eq!(coils.get(1), Some(false));
        assert_eq!(coils.get(2), None);

        let coils = Coils {
            data: &[0xff, 0b11],
            quantity: 10,
        };
        for i in 0..10 {
            assert_eq!(coils.get(i), Some(true));
        }
        assert_eq!(coils.get(11), None);
    }

    #[test]
    fn coils_iter() {
        let coils = Coils {
            data: &[0b0101_0011],
            quantity: 5,
        };
        let mut coils_iter = CoilsIter { cnt: 0, coils };
        assert_eq!(coils_iter.next(), Some(true));
        assert_eq!(coils_iter.next(), Some(true));
        assert_eq!(coils_iter.next(), Some(false));
        assert_eq!(coils_iter.next(), Some(false));
        assert_eq!(coils_iter.next(), Some(true));
        assert_eq!(coils_iter.next(), None);
    }

    #[test]
    fn coils_into_iter() {
        let coils = Coils {
            data: &[0b0101_0011],
            quantity: 3,
        };
        let mut coils_iter = coils.into_iter();
        assert_eq!(coils_iter.next(), Some(true));
        assert_eq!(coils_iter.next(), Some(true));
        assert_eq!(coils_iter.next(), Some(false));
        assert_eq!(coils_iter.next(), None);
    }

    #[test]
    fn iter_over_coils() {
        let coils = Coils {
            data: &[0b0101_0011],
            quantity: 3,
        };
        let mut cnt = 0;
        for _ in coils {
            cnt += 1;
        }
        assert_eq!(cnt, 3);
    }

    #[test]
    fn convert_bool_to_coil() {
        assert_eq!(bool_to_u16_coil(true), 0xFF00);
        assert_eq!(bool_to_u16_coil(false), 0x0000);
    }

    #[test]
    fn convert_coil_to_bool() {
        assert!(u16_coil_to_bool(0xFF00).unwrap());
        assert!(!u16_coil_to_bool(0x0000).unwrap());
        assert_eq!(
            u16_coil_to_bool(0x1234).err().unwrap(),
            Error::CoilValue(0x1234)
        );
    }

    #[test]
    fn pack_coils_into_byte_array() {
        assert_eq!(pack_coils(&[], &mut []).unwrap(), 0);
        assert_eq!(pack_coils(&[], &mut [0, 0]).unwrap(), 0);
        assert_eq!(
            pack_coils(&[true; 2], &mut []).err().unwrap(),
            Error::BufferSize
        );

        let buff = &mut [0];
        assert_eq!(pack_coils(&[true], buff).unwrap(), 1);
        assert_eq!(buff, &[0b_1]);

        let buff = &mut [0];
        assert_eq!(pack_coils(&[false], buff).unwrap(), 1);
        assert_eq!(buff, &[0b_0]);

        let buff = &mut [0];
        assert_eq!(pack_coils(&[true, false], buff).unwrap(), 1);
        assert_eq!(buff, &[0b_01]);

        let buff = &mut [0];
        assert_eq!(pack_coils(&[false, true], buff).unwrap(), 1);
        assert_eq!(buff, &[0b_10]);

        let buff = &mut [0];
        assert_eq!(pack_coils(&[true, true], buff).unwrap(), 1);
        assert_eq!(buff, &[0b_11]);

        let buff = &mut [0];
        assert_eq!(pack_coils(&[true; 8], buff).unwrap(), 1);
        assert_eq!(buff, &[0b_1111_1111]);

        let buff = &mut [0];
        assert_eq!(pack_coils(&[false; 8], buff).unwrap(), 1);
        assert_eq!(buff, &[0]);

        let buff = &mut [0, 0];
        assert_eq!(pack_coils(&[true; 9], buff).unwrap(), 2);
        assert_eq!(buff, &[0xff, 1]);
    }

    #[test]
    fn unpack_coils_from_a_byte_array() {
        assert!(unpack_coils(&[], 0, &mut []).is_ok());
        assert!(unpack_coils(&[], 0, &mut [false, false]).is_ok());
        assert!(unpack_coils(&[1, 2, 3], 0, &mut []).is_ok());
        assert_eq!(
            unpack_coils(&[], 1, &mut []).err().unwrap(),
            Error::BufferSize
        );

        let buff = &mut [false];
        assert!(unpack_coils(&[0b1], 1, buff).is_ok());
        assert_eq!(&[true], buff);

        let buff = &mut [false; 2];
        assert!(unpack_coils(&[0b01], 2, buff).is_ok());
        assert_eq!(&[true, false], buff);

        let buff = &mut [false; 2];
        assert!(unpack_coils(&[0b10], 2, buff).is_ok());
        assert_eq!(&[false, true], buff);

        let buff = &mut [false; 3];
        assert!(unpack_coils(&[0b101], 3, buff).is_ok());
        assert_eq!(&[true, false, true], buff);

        let buff = &mut [false; 10];
        assert!(unpack_coils(&[0xff, 0b11], 10, buff).is_ok());
        assert_eq!(&[true; 10], buff);
    }
}
