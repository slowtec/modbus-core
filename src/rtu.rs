//! Modbus RTU

/// Calculate the CRC (Cyclic Redundancy Check) sum.
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc = 0xFFFF;
    for x in data {
        crc ^= u16::from(*x);
        for _ in 0..8 {
            if (crc & 0x0001) != 0 {
                crc >>= 1;
                crc ^= 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    (crc << 8 | crc >> 8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_crc16() {
        let msg = &[0x01, 0x03, 0x08, 0x2B, 0x00, 0x02];
        assert_eq!(crc16(msg), 0xB663);

        let msg = &[0x01, 0x03, 0x04, 0x00, 0x20, 0x00, 0x00];
        assert_eq!(crc16(msg), 0xFBF9);
    }

}
