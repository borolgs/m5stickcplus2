use esp_hal::{gpio::Level, rmt::PulseCode};

/// Encode NEC IR command into RMT pulses
///
/// NEC Protocol timing:
/// - Leader: 9000µs mark + 4500µs space
/// - Bit 0:  560µs mark + 560µs space
/// - Bit 1:  560µs mark + 1690µs space
/// - Stop:  560µs mark
pub fn encode_nec_command(address: u8, command: u8) -> [PulseCode; 35] {
    let address_inv = !address;
    let command_inv = !command;

    let data: u32 = (address as u32)
        | ((address_inv as u32) << 8)
        | ((command as u32) << 16)
        | ((command_inv as u32) << 24);

    let mut pulses = [PulseCode::default(); 35];

    // [0] Leader: 9000µs HIGH (mark) + 4500µs LOW (space)
    pulses[0] = PulseCode::new(Level::High, 9000, Level::Low, 4500);

    // [1-32] 32 data bits
    for i in 0..32 {
        let bit = (data >> i) & 1;
        let space = if bit == 0 { 560 } else { 1690 };

        pulses[1 + i] = PulseCode::new(Level::High, 560, Level::Low, space as u16);
    }

    // [33] Stop bit: 560µs mark + 0 space
    pulses[33] = PulseCode::new(Level::High, 560, Level::Low, 0);

    // pulses[34]  PulseCode::default()

    pulses
}

/// Decode NEC protocol from received RMT pulses
pub fn decode_nec_command(pulses: &[PulseCode]) -> Option<(u8, u8)> {
    if pulses.is_empty() {
        return None;
    }

    log::info!("RX got {} pulses", pulses.len());

    for (i, p) in pulses.iter().enumerate() {
        log::info!(
            "  Pulse[{}]: L1={:?} T1={}us, L2={:?} T2={}us",
            i,
            p.level1(),
            p.length1(),
            p.level2(),
            p.length2()
        );
    }

    let first = pulses[0];
    let (_, t1, _, t2) = (
        first.level1(),
        first.length1(),
        first.level2(),
        first.length2(),
    );

    if (7200..=10800).contains(&t1) && (1800..=2700).contains(&t2) {
        log::debug!("NEC Repeat code (9ms + 2.25ms)");
        return None;
    }

    if pulses.len() < 34 {
        log::warn!("Not enough pulses: {}", pulses.len());
        return None;
    }

    if !(7200..=10800).contains(&t1) || !(3600..=5400).contains(&t2) {
        log::warn!("Invalid start pulse: {}us mark, {}us space", t1, t2);
        return None;
    }

    let mut bytes = [0u8; 4];
    for byte_idx in 0..4 {
        let mut byte = 0u8;
        for bit_idx in 0..8 {
            let pulse = pulses[1 + byte_idx * 8 + bit_idx];
            let space = pulse.length2();

            // Bit 0: ~562us space, Bit 1: ~1687us space (±20%)
            let bit = if (450..=675).contains(&space) {
                0 // Bit 0
            } else if (1350..=2025).contains(&space) {
                1 // Bit 1
            } else {
                log::warn!(
                    "Invalid bit timing: {}us at byte {} bit {}",
                    space,
                    byte_idx,
                    bit_idx
                );
                return None;
            };

            byte |= bit << bit_idx; // LSB first
        }
        bytes[byte_idx] = byte;
    }

    let address = bytes[0];
    let address_inv = bytes[1];
    let command = bytes[2];
    let command_inv = bytes[3];

    if address != !address_inv {
        log::warn!(
            "Address mismatch: 0x{:02X} vs ~0x{:02X}",
            address,
            address_inv
        );
    }
    if command != !command_inv {
        log::warn!(
            "Command mismatch: 0x{:02X} vs ~0x{:02X}",
            command,
            command_inv
        );
    }

    log::debug!(
        "📡 Decoded NEC: Address=0x{:02X}, Command=0x{:02X}",
        address,
        command
    );
    Some((address, command))
}
