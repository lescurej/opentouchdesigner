//! DMX over Art-Net and sACN (E1.31) — encode for out, decode for in.
//!
//! PLAN.md Phase 5 lists DMX as show-I/O table stakes, alongside Spout and
//! NDI. Unlike those, it needs no vendor SDK: both protocols are UDP packets
//! with a documented layout, which means the whole thing can be built and —
//! more importantly — *tested* without a lighting rig in the room.
//!
//! Channels become DMX slots in order. A channel's samples are not sent
//! individually: DMX is a state protocol, so each frame sends the current
//! value of each channel, which is the last sample of the slice.

/// Art-Net's fixed UDP port.
pub const ARTNET_PORT: u16 = 6454;
/// sACN's fixed UDP port.
pub const SACN_PORT: u16 = 5568;

/// A DMX universe: 512 slots of 8-bit level.
pub const UNIVERSE_SIZE: usize = 512;

/// Build an Art-Net `OpDmx` packet.
///
/// Layout from the Art-Net 4 specification: an 8-byte magic, a little-endian
/// opcode, a big-endian protocol version, sequence and physical bytes, a
/// little-endian universe, then a big-endian length and the slot data.
/// The mixed endianness is not a mistake in this code — it is in the spec.
pub fn artnet_packet(universe: u16, sequence: u8, slots: &[u8]) -> Vec<u8> {
    let length = slots.len().min(UNIVERSE_SIZE);
    // The spec requires an even length of at least 2.
    let padded = length.max(2).next_multiple_of(2);

    let mut p = Vec::with_capacity(18 + padded);
    p.extend_from_slice(b"Art-Net\0");
    p.extend_from_slice(&0x5000u16.to_le_bytes()); // OpDmx
    p.extend_from_slice(&14u16.to_be_bytes()); // protocol version
    p.push(sequence);
    p.push(0); // physical port, informational only
    p.extend_from_slice(&universe.to_le_bytes());
    p.extend_from_slice(&(padded as u16).to_be_bytes());
    p.extend_from_slice(&slots[..length]);
    p.resize(18 + padded, 0);
    p
}

/// Build an sACN (E1.31) data packet.
///
/// Three nested PDUs — root, framing, DMP — each carrying a flags-and-length
/// field where the top four bits are `0x7`. The layout is fiddly and the
/// offsets are easy to get wrong, which is exactly why this is a pure
/// function with a test against known byte positions.
pub fn sacn_packet(
    universe: u16,
    sequence: u8,
    source: &str,
    cid: &[u8; 16],
    slots: &[u8],
) -> Vec<u8> {
    let count = slots.len().min(UNIVERSE_SIZE);
    // The DMP layer carries a start code byte before the slots.
    let dmp_property_count = count + 1;

    let mut p = Vec::with_capacity(126 + count);

    // ---- root layer
    p.extend_from_slice(&0x0010u16.to_be_bytes()); // preamble size
    p.extend_from_slice(&0x0000u16.to_be_bytes()); // postamble size
    p.extend_from_slice(b"ASC-E1.17\0\0\0"); // ACN packet identifier
    // Each PDU length counts from its own length field to the end of the
    // packet. Getting these wrong is the classic sACN bug: receivers drop
    // the packet silently, so the test checks them against real offsets.
    let root_length = 0x7000 | (110 + count) as u16;
    p.extend_from_slice(&root_length.to_be_bytes());
    p.extend_from_slice(&0x0000_0004u32.to_be_bytes()); // VECTOR_ROOT_E131_DATA
    p.extend_from_slice(cid);

    // ---- framing layer
    let framing_length = 0x7000 | (88 + count) as u16;
    p.extend_from_slice(&framing_length.to_be_bytes());
    p.extend_from_slice(&0x0000_0002u32.to_be_bytes()); // VECTOR_E131_DATA_PACKET
    let mut name = [0u8; 64];
    for (i, b) in source.bytes().take(63).enumerate() {
        name[i] = b;
    }
    p.extend_from_slice(&name);
    p.push(100); // priority
    p.extend_from_slice(&0u16.to_be_bytes()); // synchronization address
    p.push(sequence);
    p.push(0); // options
    p.extend_from_slice(&universe.to_be_bytes());

    // ---- DMP layer
    let dmp_length = 0x7000 | (10 + dmp_property_count) as u16;
    p.extend_from_slice(&dmp_length.to_be_bytes());
    p.push(0x02); // VECTOR_DMP_SET_PROPERTY
    p.push(0xa1); // address type and data type
    p.extend_from_slice(&0x0000u16.to_be_bytes()); // first property address
    p.extend_from_slice(&0x0001u16.to_be_bytes()); // address increment
    p.extend_from_slice(&(dmp_property_count as u16).to_be_bytes());
    p.push(0x00); // DMX start code
    p.extend_from_slice(&slots[..count]);
    p
}

/// The multicast address sACN uses for a universe.
pub fn sacn_multicast(universe: u16) -> std::net::Ipv4Addr {
    let [hi, lo] = universe.to_be_bytes();
    std::net::Ipv4Addr::new(239, 255, hi, lo)
}

/// Map a channel value to a DMX slot (0–255, TouchDesigner-style).
pub fn to_slot(value: f32) -> u8 {
    if value.is_nan() {
        return 0;
    }
    (value.clamp(0.0, 255.0) + 0.5) as u8
}

pub fn from_slot(slot: u8) -> f32 {
    f32::from(slot)
}

pub fn parse_artnet_dmx(buf: &[u8]) -> Option<(u16, &[u8])> {
    if buf.len() < 18 || &buf[0..8] != b"Art-Net\0" {
        return None;
    }
    if u16::from_le_bytes([buf[8], buf[9]]) != 0x5000 {
        return None;
    }
    let universe = u16::from_le_bytes([buf[14], buf[15]]);
    let length = u16::from_be_bytes([buf[16], buf[17]]) as usize;
    let data_end = (18 + length).min(buf.len());
    if data_end <= 18 {
        return None;
    }
    Some((universe, &buf[18..data_end]))
}

pub fn parse_sacn_dmx(buf: &[u8]) -> Option<(u16, &[u8])> {
    if buf.len() < 126 || &buf[4..16] != b"ASC-E1.17\0\0\0" {
        return None;
    }
    let universe = u16::from_be_bytes([buf[113], buf[114]]);
    let property_count = u16::from_be_bytes([buf[123], buf[124]]) as usize;
    if property_count == 0 {
        return None;
    }
    let data_len = property_count.saturating_sub(1);
    let end = 126 + data_len;
    if end > buf.len() {
        return None;
    }
    Some((universe, &buf[126..end]))
}

#[cfg(test)]
#[path = "tests/dmx__tests.rs"]
mod tests;
