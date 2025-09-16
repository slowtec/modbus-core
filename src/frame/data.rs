// SPDX-FileCopyrightText: Copyright (c) 2018-2025 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::*;
use crate::error::*;

/// Modbus data (u16 values)
#[cfg_attr(all(feature = "defmt", target_os = "none"), derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Data<'d> {
    pub(crate) data: RawData<'d>,
    pub(crate) quantity: usize,
}

impl<'d> Data<'d> {
    /// Pack words (u16 values) into a byte buffer.
    pub fn from_words(words: &[u16], target: &'d mut [u8]) -> Result<Self, Error> {
        if (words.len() * 2 > target.len()) || words.is_empty() {
            return Err(Error::BufferSize);
        }
        for (i, w) in words.iter().enumerate() {
            BigEndian::write_u16(&mut target[i * 2..], *w);
        }
        Ok(Data {
            data: target,
            quantity: words.len(),
        })
    }
    //TODO: add tests
    pub(crate) fn copy_to(&self, buf: &mut [u8]) {
        let cnt = self.quantity * 2;
        debug_assert!(buf.len() >= cnt);
        (0..cnt).for_each(|idx| {
            buf[idx] = self.data[idx];
        });
    }
    /// Quantity of words (u16 values)
    #[must_use]
    pub const fn len(&self) -> usize {
        self.quantity
    }
    ///  Returns `true` if the container has no items.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.quantity == 0
    }
    /// Get a specific word.
    #[must_use]
    pub fn get(&self, idx: usize) -> Option<Word> {
        if idx + 1 > self.quantity {
            return None;
        }
        let idx = idx * 2;
        Some(BigEndian::read_u16(&self.data[idx..idx + 2]))
    }

    #[must_use]
    pub const fn payload(&self) -> &[u8] {
        self.data
    }
}

/// Data iterator
// TODO: crate a generic iterator
#[cfg_attr(all(feature = "defmt", target_os = "none"), derive(defmt::Format))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataIter<'d> {
    cnt: usize,
    data: Data<'d>,
}

impl Iterator for DataIter<'_> {
    type Item = Word;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.data.get(self.cnt);
        self.cnt += 1;
        result
    }
}

impl<'d> IntoIterator for Data<'d> {
    type Item = Word;
    type IntoIter = DataIter<'d>;

    fn into_iter(self) -> Self::IntoIter {
        DataIter { cnt: 0, data: self }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn from_word_slice() {
        let words: &[u16] = &[0xABCD, 0xEF00, 0x1234];
        let buff: &mut [u8] = &mut [0; 5];
        assert!(Data::from_words(words, buff).is_err());
        let buff: &mut [u8] = &mut [0; 6];
        let data = Data::from_words(words, buff).unwrap();
        assert_eq!(data.len(), 3);
        let mut iter = data.into_iter();
        assert_eq!(iter.next(), Some(0xABCD));
        assert_eq!(iter.next(), Some(0xEF00));
        assert_eq!(iter.next(), Some(0x1234));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn data_len() {
        let data = Data {
            data: &[0, 1, 2],
            quantity: 5,
        };
        assert_eq!(data.len(), 5);
    }

    #[test]
    fn data_empty() {
        let data = Data {
            data: &[0, 1, 2],
            quantity: 0,
        };
        assert!(data.is_empty());
    }

    #[test]
    fn data_get() {
        let data = Data {
            data: &[0xAB, 0xBC, 0x12],
            quantity: 1,
        };
        assert_eq!(data.get(0), Some(0xABBC));
        assert_eq!(data.get(1), None);

        let data = Data {
            data: &[0xFF, 0xAB, 0xCD, 0xEF, 0x33],
            quantity: 2,
        };
        assert_eq!(data.get(0), Some(0xFFAB));
        assert_eq!(data.get(1), Some(0xCDEF));
        assert_eq!(data.get(2), None);
    }

    #[test]
    fn data_iter() {
        let data = Data {
            data: &[0x01, 0x02, 0x03, 0x04, 0xAA, 0xBB],
            quantity: 3,
        };
        let mut data_iter = DataIter { cnt: 0, data };
        assert_eq!(data_iter.next(), Some(0x0102));
        assert_eq!(data_iter.next(), Some(0x0304));
        assert_eq!(data_iter.next(), Some(0xAABB));
        assert_eq!(data_iter.next(), None);
    }

    #[test]
    fn data_into_iter() {
        let data = Data {
            data: &[0x01, 0x02, 0x03, 0x04, 0xAA, 0xBB],
            quantity: 3,
        };
        let mut data_iter = data.into_iter();
        assert!(data_iter.next().is_some());
        assert!(data_iter.next().is_some());
        assert!(data_iter.next().is_some());
        assert!(data_iter.next().is_none());
    }
}
