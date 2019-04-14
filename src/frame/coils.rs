use super::*;
use crate::{error::*, util::*};

/// Packed coils
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Coils<'c> {
    pub(crate) data: RawData<'c>,
    pub(crate) quantity: usize,
}

impl<'c> Coils<'c> {
    /// Pack coils defined by an bool slice into a byte buffer.
    pub fn from_bools(bools: &[bool], target: &'c mut [u8]) -> Result<Self, Error> {
        pack_coils(bools, target)?;
        Ok(Coils {
            data: target,
            quantity: bools.len(),
        })
    }
    /// Quantity of coils
    pub const fn len(&self) -> usize {
        self.quantity
    }
    ///  Returns `true` if the container has no items.
    pub const fn is_empty(&self) -> bool {
        self.quantity == 0
    }
    /// Get a specific coil.
    pub fn get(&self, idx: usize) -> Option<Coil> {
        if idx + 1 > self.quantity {
            return None;
        }
        Some((self.data[(idx as u16 / 8u16) as usize] >> (idx % 8)) & 0b1 > 0)
    }
}

/// Coils iterator.
// TODO: crate an generic iterator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoilsIter<'c> {
    cnt: usize,
    coils: Coils<'c>,
}

impl<'c> Iterator for CoilsIter<'c> {
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
}
