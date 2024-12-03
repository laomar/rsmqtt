pub mod pingreq {
    use bytes::{BufMut, BytesMut};
    use crate::packet::{PacketType};

    pub fn write(write: &mut BytesMut) {
        write.put_u8((PacketType::PingReq as u8) << 4);
        write.put_u8(0x00);
    }
}

pub mod pingresp {
    use bytes::{BufMut, BytesMut};
    use crate::packet::{PacketType};

    pub fn write(write: &mut BytesMut) {
        write.put_u8((PacketType::PingResp as u8) << 4);
        write.put_u8(0x00);
    }
}