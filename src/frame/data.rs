use super::*;

/// Modbus data (u16 values)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Data<'d> {
    pub(crate) data: RawData<'d>,
    pub(crate) quantity: usize,
}

impl<'d> Data<'d> {
    /// Quantity of words (u16 values)
    pub const fn len(&self) -> usize {
        self.quantity
    }
    ///  Returns `true` if the container has no items.
    pub const fn is_empty(&self) -> bool {
        self.quantity == 0
    }
    /// Get a specific word.
    pub fn get(&self, idx: usize) -> Option<Word> {
        if idx + 1 > self.quantity {
            return None;
        }
        let idx = idx * 2;
        Some(BigEndian::read_u16(&self.data[idx..idx + 2]))
    }
}

/// Data iterator
// TODO: crate a generic iterator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataIter<'d> {
    cnt: usize,
    data: Data<'d>,
}

impl<'d> Iterator for DataIter<'d> {
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
