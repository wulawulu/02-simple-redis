use crate::{Backend, RespArray, RespFrame};

use super::{AddMember, CommandError, CommandExecutor, SisMember, extract_args, validate_command};

impl CommandExecutor for AddMember {
    fn execute(self, backend: &Backend) -> RespFrame {
        backend.add_member(self.key, self.member);
        RespFrame::Integer(1)
    }
}

impl CommandExecutor for SisMember {
    fn execute(self, backend: &Backend) -> RespFrame {
        backend.sis_member(self.key, self.member)
    }
}

impl TryFrom<RespArray> for AddMember {
    type Error = CommandError;

    fn try_from(value: RespArray) -> Result<Self, Self::Error> {
        validate_command(&value, &["addmember"], 2)?;

        let mut args = extract_args(value, 1)?.into_iter();
        match (args.next(), args.next()) {
            (Some(RespFrame::BulkString(key)), Some(RespFrame::BulkString(member))) => {
                Ok(AddMember {
                    key: String::from_utf8(key.0)?,
                    member: String::from_utf8(member.0)?,
                })
            }
            _ => Err(CommandError::InvalidCommand("Invalid command".to_string())),
        }
    }
}

impl TryFrom<RespArray> for SisMember {
    type Error = CommandError;

    fn try_from(value: RespArray) -> Result<Self, Self::Error> {
        validate_command(&value, &["sismember"], 2)?;

        let mut args = extract_args(value, 1)?.into_iter();
        match (args.next(), args.next()) {
            (Some(RespFrame::BulkString(key)), Some(RespFrame::BulkString(member))) => {
                Ok(SisMember {
                    key: String::from_utf8(key.0)?,
                    member: String::from_utf8(member.0)?,
                })
            }
            _ => Err(CommandError::InvalidCommand("Invalid command".to_string())),
        }
    }
}
