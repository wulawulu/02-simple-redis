use crate::{Backend, RespArray, RespFrame};

use super::{CommandError, CommandExecutor, Echo, extract_args, validate_command};

impl CommandExecutor for Echo {
    fn execute(self, _: &Backend) -> RespFrame {
        RespFrame::BulkString(self.message.into())
    }
}

impl TryFrom<RespArray> for Echo {
    type Error = CommandError;

    fn try_from(value: RespArray) -> Result<Self, Self::Error> {
        validate_command(&value, &["echo"], 1)?;

        let mut args = extract_args(value, 1)?.into_iter();
        match args.next() {
            Some(RespFrame::BulkString(key)) => Ok(Echo {
                message: String::from_utf8(key.0)?,
            }),
            _ => Err(CommandError::InvalidCommand("Invalid command".to_string())),
        }
    }
}
