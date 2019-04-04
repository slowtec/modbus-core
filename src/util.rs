//! Common helpers

use super::*;

/// Turn a bool into a u16 coil value
pub fn bool_to_u16_coil(state: bool) -> u16 {
    if state {
        0xFF00
    } else {
        0x0000
    }
}

/// Turn a u16 coil value into a boolean value.
pub fn u16_coil_to_bool(coil: u16) -> Result<bool, Error> {
    match coil {
        0xFF00 => Ok(true),
        0x0000 => Ok(false),
        _ => Err(Error::CoilValue(coil)),
    }
}

/// Calculate the number of bytes required for a given number of coils.
pub const fn packed_coils_len(bitcount: usize) -> usize {
    (bitcount + 7) / 8
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
        let v = if *b { 0b1 } else { 0b0 };
        bytes[(i / 8) as usize] |= v << (i % 8);
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
    fn convert_bool_to_coil() {
        assert_eq!(bool_to_u16_coil(true), 0xFF00);
        assert_eq!(bool_to_u16_coil(false), 0x0000);
    }

    #[test]
    fn convert_coil_to_bool() {
        assert_eq!(u16_coil_to_bool(0xFF00).unwrap(), true);
        assert_eq!(u16_coil_to_bool(0x0000).unwrap(), false);
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
