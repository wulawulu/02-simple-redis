mod echo;
mod hmap;
mod map;
mod set;

use enum_dispatch::enum_dispatch;
use lazy_static::lazy_static;
use thiserror::Error;

use crate::RespArray;
use crate::SimpleString;
use crate::{backend::Backend, RespError, RespFrame};

lazy_static! {
    static ref RESP_OK: RespFrame = SimpleString::from("OK").into();
}

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("Invalid command: {0}")]
    InvalidCommand(String),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("{0}")]
    RespError(#[from] RespError),
    #[error("Uft8 error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}

#[enum_dispatch]
pub trait CommandExecutor {
    fn execute(self, backend: &Backend) -> RespFrame;
}

#[derive(Debug)]
#[enum_dispatch(CommandExecutor)]
pub enum Command {
    Get(Get),
    Set(Set),
    HGet(HGet),
    HMGet(HMGet),
    HSet(HSet),
    HGetAll(HGetAll),
    Unrecognized(Unrecognized),
    Echo(Echo),
    SisMember(SisMember),
    AddMember(AddMember),
}

#[derive(Debug)]
pub struct SisMember {
    pub key: String,
    pub member: String,
}

#[derive(Debug)]
pub struct AddMember {
    pub key: String,
    pub member: String,
}

#[derive(Debug)]
pub struct Echo {
    pub message: String,
}

#[derive(Debug)]
pub struct Unrecognized;

impl CommandExecutor for Unrecognized {
    fn execute(self, _: &Backend) -> RespFrame {
        RESP_OK.clone()
    }
}

#[derive(Debug)]
pub struct Get {
    pub key: String,
}

#[derive(Debug)]
pub struct Set {
    pub key: String,
    pub value: RespFrame,
}

#[derive(Debug)]
pub struct HGet {
    pub key: String,
    pub field: String,
}

#[derive(Debug)]
pub struct HMGet {
    pub key: String,
    pub fields: Vec<String>,
}

#[derive(Debug)]
pub struct HSet {
    pub key: String,
    pub field: String,
    pub value: RespFrame,
}

#[derive(Debug)]
pub struct HGetAll {
    pub key: String,
    pub sort: bool,
}

impl TryFrom<RespFrame> for Command {
    type Error = CommandError;

    fn try_from(value: RespFrame) -> Result<Self, Self::Error> {
        match value {
            RespFrame::Array(resp_array) => resp_array.try_into(),
            _ => Err(CommandError::InvalidCommand(
                "Command must be an Array".to_string(),
            )),
        }
    }
}

impl TryFrom<RespArray> for Command {
    type Error = CommandError;

    fn try_from(value: RespArray) -> Result<Self, Self::Error> {
        match value.first() {
            Some(RespFrame::BulkString(ref cmd)) => match cmd.as_ref() {
                b"echo" => Ok(Echo::try_from(value)?.into()),
                b"get" => Ok(Get::try_from(value)?.into()),
                b"set" => Ok(Set::try_from(value)?.into()),
                b"hget" => Ok(HGet::try_from(value)?.into()),
                b"hmget" => Ok(HMGet::try_from(value)?.into()),
                b"hset" => Ok(HSet::try_from(value)?.into()),
                b"hgetall" => Ok(HGetAll::try_from(value)?.into()),
                b"addmember" => Ok(AddMember::try_from(value)?.into()),
                b"sismember" => Ok(SisMember::try_from(value)?.into()),
                _ => Ok(Unrecognized.into()),
            },
            _ => Err(CommandError::InvalidCommand(
                "Command must have a BulkString as the first argument".to_string(),
            )),
        }
    }
}

fn validate_command(
    frames: &RespArray,
    cmds: &[&'static str],
    arg_cnt: usize,
) -> Result<(), CommandError> {
    if frames.len() < cmds.len() + arg_cnt {
        return Err(CommandError::InvalidCommand(format!(
            "{} command must have at least {} argument",
            cmds.join(" "),
            arg_cnt
        )));
    }
    for (i, cmd) in cmds.iter().enumerate() {
        match frames[i] {
            RespFrame::BulkString(ref frame_value) => {
                if frame_value.as_ref().to_ascii_lowercase() != cmd.as_bytes() {
                    return Err(CommandError::InvalidCommand(format!(
                        "Invalid command: expected {}, got {}",
                        cmd,
                        String::from_utf8_lossy(frame_value.as_ref())
                    )));
                }
            }
            _ => {
                return Err(CommandError::InvalidCommand(
                    "Command must have a BulkString as the first argument".to_string(),
                ));
            }
        }
    }
    Ok(())
}

fn extract_args(frames: RespArray, start: usize) -> Result<Vec<RespFrame>, CommandError> {
    Ok(frames.0.into_iter().skip(start).collect::<Vec<RespFrame>>())
}

#[cfg(test)]
mod tests {
    use crate::RespDecode;

    use super::*;
    use crate::RespNull;
    use anyhow::Result;
    use bytes::BytesMut;

    #[test]
    fn test_command() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"*2\r\n$3\r\nget\r\n$5\r\nhello\r\n");

        let frame = RespArray::decode(&mut buf)?;
        let cmd: Command = frame.try_into()?;

        let backend = Backend::new();
        let ret = cmd.execute(&backend);
        assert_eq!(ret, RespFrame::Null(RespNull));

        Ok(())
    }
}
