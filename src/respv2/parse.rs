use winnow::{
    Parser,
    ascii::{digit1, float},
    combinator::{alt, dispatch, fail, opt, preceded, terminated},
    error::ContextError,
    token::{any, take, take_till, take_until},
};

use crate::{
    BulkString, RespArray, RespError, RespFrame, RespMap, RespNull, RespSet, SimpleError,
    SimpleString,
};
use winnow::Result;

const CRLF: &[u8] = b"\r\n";

pub fn parse_frame_length(input: &[u8]) -> Result<usize, RespError> {
    let target = &mut (&*input);
    let ret = advance(target);

    match ret {
        Ok(_) => {
            let start = input.as_ptr() as usize;
            let end = (*target).as_ptr() as usize;
            Ok(end - start)
        }
        Err(_) => Err(RespError::NotComplete),
    }
}

fn advance(input: &mut &[u8]) -> Result<()> {
    let mut simple_advance = terminated(take_until(0.., CRLF), CRLF).value(());
    dispatch! {any;
        b'+' => simple_advance,
        b'-' => simple_advance,
        b':' => simple_advance,
        b'$' => bulk_string_advance,
        b'*' => array_advance,
        b'_' => simple_advance,
        b'#' => simple_advance,
        b',' => simple_advance,
        b'%' => map_advance,
        b'~' => set_advance,
        _v=>fail::<_,_,_>,
    }
    .parse_next(input)
}

fn bulk_string_advance(input: &mut &[u8]) -> Result<()> {
    let len = integer.parse_next(input)?;
    if len == -1 || len == 0 {
        return Ok(());
    } else if len < 0 {
        return Err(err_cut("bulk string length must be non-negative"));
    }
    terminated(take(len as usize), CRLF).parse_next(input)?;
    Ok(())
}

fn array_advance(input: &mut &[u8]) -> Result<()> {
    let len = integer.parse_next(input)?;
    if len == -1 || len == 0 {
        return Ok(());
    }
    for _ in 0..len {
        advance(input)?;
    }
    Ok(())
}

fn map_advance(input: &mut &[u8]) -> Result<()> {
    let len = integer.parse_next(input)?;
    if len == -1 || len == 0 {
        return Ok(());
    }
    for _ in 0..len {
        terminated(take_till(0.., CRLF), CRLF)
            .value(())
            .parse_next(input)?;
        advance(input)?;
    }
    Ok(())
}

fn set_advance(input: &mut &[u8]) -> Result<()> {
    let len = integer.parse_next(input)?;
    if len == -1 || len == 0 {
        return Ok(());
    }
    for _ in 0..len {
        advance(input)?;
    }
    Ok(())
}

pub fn parse_frame(input: &mut &[u8]) -> Result<RespFrame> {
    dispatch! {any;
        b'+' => simple_string.map(RespFrame::SimpleString),
        b'-' => simple_error.map(RespFrame::Error),
        b':' => integer.map(RespFrame::Integer),
        b'$' => bulk_string.map(RespFrame::BulkString),
        b'*' => array.map(RespFrame::Array),
        b'_' => null.map(RespFrame::Null),
        b'#' => boolean.map(RespFrame::Boolean),
        b',' => double.map(RespFrame::Double),
        b'%' => map.map(RespFrame::Map),
        b'~' => set.map(RespFrame::Set),
        _ => fail::<_,_,_>,
    }
    .parse_next(input)
}

fn simple_string(input: &mut &[u8]) -> Result<SimpleString> {
    parse_string.map(SimpleString).parse_next(input)
}

fn simple_error(input: &mut &[u8]) -> Result<SimpleError> {
    parse_string.map(SimpleError).parse_next(input)
}

fn integer(input: &mut &[u8]) -> Result<i64> {
    let sign = opt(alt(('+', '-'))).parse_next(input)?.unwrap_or('+');
    let sign = if sign == '+' { 1 } else { -1 };
    let digits: i64 = terminated(digit1.parse_to(), CRLF).parse_next(input)?;
    Ok(sign * digits)
}

fn bulk_string(input: &mut &[u8]) -> Result<BulkString> {
    let len = terminated(digit1.parse_to::<i64>(), CRLF).parse_next(input)?;
    if len == -1 {
        return Ok(BulkString::new(vec![]));
    }

    let data = terminated(take(len as usize), CRLF).parse_next(input)?;
    Ok(BulkString::new(data))
}

fn array(input: &mut &[u8]) -> Result<RespArray> {
    let len = terminated(digit1.parse_to::<i64>(), CRLF).parse_next(input)?;
    if len == -1 {
        return Ok(RespArray::new(vec![]));
    }

    let mut items = Vec::new();
    for _ in 0..len {
        items.push(parse_frame(input)?);
    }
    Ok(RespArray::new(items))
}

fn null(input: &mut &[u8]) -> Result<RespNull> {
    b"\r\n".parse_next(input).map(|_| RespNull)
}

fn boolean(input: &mut &[u8]) -> Result<bool> {
    terminated(alt(('t', 'f')), CRLF)
        .parse_next(input)
        .map(|s| s == 't')
}

fn double(input: &mut &[u8]) -> Result<f64> {
    terminated(float, CRLF).parse_next(input)
}

fn map(input: &mut &[u8]) -> Result<RespMap> {
    let len = terminated(digit1.parse_to::<i64>(), CRLF).parse_next(input)?;
    let mut map = RespMap::new();

    for _ in 0..len {
        let key = preceded('+', parse_string).parse_next(input)?;
        let value = parse_frame(input)?;
        map.insert(key, value);
    }
    Ok(map)
}

fn set(input: &mut &[u8]) -> Result<RespSet> {
    let len = terminated(digit1.parse_to::<i64>(), CRLF).parse_next(input)?;
    let len = len / 2;
    let mut items = Vec::new();
    for _ in 0..len {
        let item = parse_frame(input)?;
        items.push(item);
    }
    Ok(RespSet::new(items))
}

fn parse_string(input: &mut &[u8]) -> Result<String> {
    terminated(take_till(0.., CRLF), CRLF)
        .map(|s: &[u8]| String::from_utf8_lossy(s).into_owned())
        .parse_next(input)
}

fn err_cut(_s: impl Into<String>) -> ContextError {
    ContextError::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bulk_string_len() {
        let input = b"$4\r\nping\r\n";
        let input = &input[..];
        let len = parse_frame_length(input).unwrap();
        assert_eq!(input.len(), len);
    }

    #[test]
    fn test_array_len() {
        let input = b"*2\r\n$4\r\nping\r\n$4\r\npong\r\n";
        let input = &input[..];
        let len = parse_frame_length(input).unwrap();
        assert_eq!(input.len(), len);
    }

    #[test]
    fn test_map_len() {
        let input = b"%2\r\n+first\r\n:1\r\n+second\r\n:2\r\n";
        let input = &input[..];
        let len = parse_frame_length(input);
        match len {
            Ok(len) => assert_eq!(input.len(), len),
            Err(e) => panic!("{}", e),
        }
    }

    #[test]
    fn test_set_len() {
        let input = b"~2\r\n$4\r\nping\r\n$4\r\npong\r\n";
        let input = &input[..];
        let len = parse_frame_length(input).unwrap();
        assert_eq!(input.len(), len);
    }
}
