use bytes::BytesMut;

use crate::{RespError, RespFrame};

mod parse;

pub use self::parse::{parse_frame, parse_frame_length};

pub trait RespDecodeV2: Sized {
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError>;
    fn expect_length(buf: &[u8]) -> Result<usize, RespError>;
}

impl RespDecodeV2 for RespFrame {
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let len = Self::expect_length(buf)?;
        let data = buf.split_to(len);
        parse::parse_frame(&mut data.as_ref()).map_err(|e| RespError::InvalidFrame(e.to_string()))
    }

    fn expect_length(buf: &[u8]) -> Result<usize, RespError> {
        parse::parse_frame_length(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn respv2_simple_string_length_should_work() {
        let buf = b"+OK\r\n";
        let len = RespFrame::expect_length(buf).unwrap();
        assert_eq!(len, buf.len());
    }

    #[test]
    fn respv2_simple_string_bad_length_should_fail() {
        let buf = b"+OK\r";
        let err = RespFrame::expect_length(buf).unwrap_err();
        assert_eq!(err, RespError::NotComplete);
    }

    #[test]
    fn respv2_simple_string_should_work() {
        let mut buf = BytesMut::from("+OK\r\n");
        let frame = RespFrame::decode(&mut buf).unwrap();
        assert_eq!(frame, RespFrame::SimpleString("OK".into()));
    }

    #[test]
    fn respv2_simple_error_length_should_work() {
        let buf = b"-ERR\r\n";
        let len = RespFrame::expect_length(buf).unwrap();
        assert_eq!(len, buf.len());
    }

    #[test]
    fn respv2_simple_error_should_work() {
        let mut buf = BytesMut::from("-ERR\r\n");
        let frame = RespFrame::decode(&mut buf).unwrap();
        assert_eq!(frame, RespFrame::Error("ERR".into()));
    }

    #[test]
    fn respv2_integer_length_should_work() {
        let buf = b":1000\r\n";
        let len = RespFrame::expect_length(buf).unwrap();
        assert_eq!(len, buf.len());
    }

    #[test]
    fn respv2_integer_should_work() {
        let mut buf = BytesMut::from(":1000\r\n");
        let frame = RespFrame::decode(&mut buf).unwrap();
        assert_eq!(frame, RespFrame::Integer(1000));
    }

    #[test]
    fn respv2_bulk_string_length_should_work() {
        let buf = b"$6\r\nfoobar\r\n";
        let len = RespFrame::expect_length(buf).unwrap();
        assert_eq!(len, buf.len());
    }

    #[test]
    fn respv2_bulk_string_should_work() {
        let mut buf = BytesMut::from("$6\r\nfoobar\r\n");
        let frame = RespFrame::decode(&mut buf).unwrap();
        assert_eq!(frame, RespFrame::BulkString("foobar".into()));
    }

    #[test]
    fn respv2_null_bulk_string_length_should_work() {
        let buf = b"$-1\r\n";
        let len = RespFrame::expect_length(buf).unwrap();
        assert_eq!(len, buf.len());
    }

    #[test]
    fn respv2_array_length_should_work() {
        let buf = b"*2\r\n+OK\r\n-ERR\r\n";
        let len = RespFrame::expect_length(buf).unwrap();
        assert_eq!(len, buf.len());
    }

    #[test]
    fn respv2_array_should_work() {
        let mut buf = BytesMut::from("*2\r\n+OK\r\n-ERR\r\n");
        let frame = RespFrame::decode(&mut buf).unwrap();
        assert_eq!(
            frame,
            RespFrame::Array(
                vec![
                    RespFrame::SimpleString("OK".into()),
                    RespFrame::Error("ERR".into())
                ]
                .into()
            )
        );
    }

    #[test]
    fn respv2_null_array_length_should_work() {
        let buf = b"*-1\r\n";
        let len = RespFrame::expect_length(buf).unwrap();
        assert_eq!(len, buf.len());
    }

    #[test]
    fn respv2_map_length_should_work() {
        let buf = b"%1\r\n+OK\r\n-ERR\r\n";
        let len = RespFrame::expect_length(buf).unwrap();
        assert_eq!(len, buf.len());
    }

    #[test]
    fn respv2_map_should_work() {
        let mut buf = BytesMut::from("%1\r\n+OK\r\n-ERR\r\n");
        let frame = RespFrame::decode(&mut buf).unwrap();
        let items: BTreeMap<String, RespFrame> =
            [("OK".to_string(), RespFrame::Error("ERR".into()))]
                .into_iter()
                .collect();
        assert_eq!(frame, RespFrame::Map(items.into()));
    }
}
