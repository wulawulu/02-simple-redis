use crate::resp::{BUF_CAP, CRLF_LEN, calc_total_length, extract_fixed_data, parse_length};
use crate::{RespDecode, RespEncode, RespError, RespFrame};
use bytes::{Buf, BytesMut};
use std::ops::Deref;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct RespArray(pub(crate) Vec<RespFrame>);

impl RespArray {
    pub fn new(s: impl Into<Vec<RespFrame>>) -> Self {
        RespArray(s.into())
    }
}

impl Deref for RespArray {
    type Target = Vec<RespFrame>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// - array: "*<number-of-elements>\r\n<element-1>...<element-n>"
impl RespEncode for RespArray {
    fn encode(self) -> Vec<u8> {
        if self.0.is_empty() {
            return b"*-1\r\n".to_vec();
        }
        let mut buf = Vec::with_capacity(BUF_CAP);
        buf.extend_from_slice(&format!("*{}\r\n", self.0.len()).into_bytes());
        for frame in self.0 {
            buf.extend_from_slice(&frame.encode());
        }
        buf
    }
}

impl RespDecode for RespArray {
    const PREFIX: &'static str = "*";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        if extract_fixed_data(buf, "*-1\r\n", "NullArray").is_ok() {
            return Ok(RespArray::new(vec![]));
        }
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        let total_len = calc_total_length(buf, end, len, Self::PREFIX)?;

        if buf.len() < total_len {
            return Err(RespError::NotComplete);
        }

        buf.advance(end + CRLF_LEN);

        let mut frames = Vec::with_capacity(len);
        for _ in 0..len {
            frames.push(RespFrame::decode(buf)?);
        }

        Ok(RespArray::new(frames))
    }
    fn expect_length(buf: &[u8]) -> Result<usize, RespError> {
        if buf.starts_with(b"*-1\r\n") {
            return Ok(5);
        }
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        calc_total_length(buf, end, len, Self::PREFIX)
    }
}

impl From<Vec<RespFrame>> for RespArray {
    fn from(s: Vec<RespFrame>) -> Self {
        RespArray(s)
    }
}

#[cfg(test)]
mod tests {
    use crate::{BulkString, RespArray, RespDecode, RespEncode, RespError, RespFrame};
    use bytes::BytesMut;

    #[test]
    fn test_null_array_decode() -> anyhow::Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"*-1\r\n");

        let frame = RespArray::decode(&mut buf)?;
        assert_eq!(frame, RespArray::new(vec![]));

        Ok(())
    }

    #[test]
    fn test_array_decode() -> anyhow::Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"*2\r\n$3\r\nset\r\n$5\r\nhello\r\n");

        let frame = RespArray::decode(&mut buf)?;
        assert_eq!(frame, RespArray::new([b"set".into(), b"hello".into()]));

        buf.extend_from_slice(b"*2\r\n$3\r\nset\r\n");
        let ret = RespArray::decode(&mut buf);
        assert_eq!(ret.unwrap_err(), RespError::NotComplete);
        buf.extend_from_slice(b"$5\r\nhello\r\n");
        let frame = RespArray::decode(&mut buf)?;
        assert_eq!(frame, RespArray::new([b"set".into(), b"hello".into()]));
        Ok(())
    }

    #[test]
    fn test_array_encode() {
        let frame: RespFrame = RespArray::new(vec![
            BulkString::new("set".to_string()).into(),
            BulkString::new("hello".to_string()).into(),
            BulkString::new("world".to_string()).into(),
        ])
        .into();
        assert_eq!(
            &frame.encode(),
            b"*3\r\n$3\r\nset\r\n$5\r\nhello\r\n$5\r\nworld\r\n"
        );
    }

    #[test]
    fn test_null_array_encode() {
        let frame: RespFrame = RespArray::new(vec![]).into();
        assert_eq!(frame.encode(), b"*-1\r\n");
    }
}
