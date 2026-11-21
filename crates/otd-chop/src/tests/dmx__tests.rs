use super::*;

#[test]
fn an_artnet_packet_has_the_documented_layout() {
    let p = artnet_packet(3, 7, &[255, 0, 128]);
    assert_eq!(&p[0..8], b"Art-Net\0");
    // Opcode is little-endian, protocol version big-endian.
    assert_eq!(&p[8..10], &[0x00, 0x50]);
    assert_eq!(&p[10..12], &[0x00, 0x0e]);
    assert_eq!(p[12], 7, "sequence");
    assert_eq!(&p[14..16], &[3, 0], "universe, little-endian");
    assert_eq!(&p[16..18], &[0, 4], "length, big-endian and even");
    assert_eq!(&p[18..22], &[255, 0, 128, 0]);
}

#[test]
fn an_artnet_packet_is_never_odd_or_shorter_than_two() {
    assert_eq!(artnet_packet(0, 0, &[]).len(), 18 + 2);
    assert_eq!(artnet_packet(0, 0, &[1]).len(), 18 + 2);
    assert_eq!(artnet_packet(0, 0, &[1, 2, 3]).len(), 18 + 4);
    // And a universe cannot exceed 512 slots however many channels exist.
    let big = vec![1u8; 900];
    assert_eq!(artnet_packet(0, 0, &big).len(), 18 + 512);
}

#[test]
fn an_sacn_packet_has_the_documented_layout() {
    let cid = [0xab; 16];
    let p = sacn_packet(1, 5, "otd", &cid, &[10, 20, 30]);
    assert_eq!(&p[0..2], &[0x00, 0x10], "preamble");
    assert_eq!(&p[4..16], b"ASC-E1.17\0\0\0");
    assert_eq!(&p[22..38], &cid);
    assert_eq!(&p[44..47], b"otd");
    assert_eq!(p[108], 100, "priority");
    assert_eq!(p[111], 5, "sequence");
    assert_eq!(&p[113..115], &[0, 1], "universe, big-endian");
    assert_eq!(p[125], 0x00, "DMX start code");
    assert_eq!(&p[126..129], &[10, 20, 30]);

    // Every PDU length carries the 0x7 flags nibble and counts from its
    // own field to the end of the packet.
    for (offset, label) in [(16usize, "root"), (38, "framing"), (115, "dmp")] {
        let len = u16::from_be_bytes([p[offset], p[offset + 1]]);
        assert_eq!(len >> 12, 0x7, "{label} flags");
        assert_eq!(len & 0x0fff, (p.len() - offset) as u16, "{label} length");
    }
}

#[test]
fn sacn_multicast_follows_the_universe() {
    assert_eq!(sacn_multicast(1), std::net::Ipv4Addr::new(239, 255, 0, 1));
    assert_eq!(
        sacn_multicast(300),
        std::net::Ipv4Addr::new(239, 255, 1, 44)
    );
}

#[test]
fn levels_saturate_rather_than_wrapping() {
    assert_eq!(to_slot(0.0), 0);
    assert_eq!(to_slot(255.0), 255);
    assert_eq!(to_slot(128.0), 128);
    assert_eq!(to_slot(300.0), 255);
    assert_eq!(to_slot(-0.3), 0);
    assert_eq!(to_slot(f32::NAN), 0);
}

#[test]
fn artnet_packets_round_trip_through_the_parser() {
    let raw = artnet_packet(2, 1, &[255, 128, 0]);
    let (u, slots) = parse_artnet_dmx(&raw).unwrap();
    assert_eq!(u, 2);
    assert_eq!(slots, &[255, 128, 0, 0]);
}

#[test]
fn sacn_packets_round_trip_through_the_parser() {
    let cid = [1u8; 16];
    let raw = sacn_packet(1, 2, "otd", &cid, &[10, 20]);
    let (u, slots) = parse_sacn_dmx(&raw).unwrap();
    assert_eq!(u, 1);
    assert_eq!(slots, &[10, 20]);
}
