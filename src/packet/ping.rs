pub mod pingreq {
    use crate::packet::PacketType;
    use bytes::{BufMut, BytesMut};

    pub fn pack(write: &mut BytesMut) {
        write.put_u8((PacketType::PingReq as u8) << 4);
        write.put_u8(0x00);
    }
}

pub mod pingresp {
    use crate::packet::PacketType;
    use bytes::{BufMut, BytesMut};

    pub fn pack(write: &mut BytesMut) {
        write.put_u8((PacketType::PingResp as u8) << 4);
        write.put_u8(0x00);
    }
}
