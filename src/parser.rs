use core::str::FromStr;
use heapless::String;

#[derive(Debug)]
pub enum Command {
    Ping { payload: String<16> },
    Led { on: bool },
    Version,
}

impl TryFrom<String<128>> for Command {
    type Error = &'static str;

    fn try_from(value: String<128>) -> Result<Self, Self::Error> {
        let mut tokens = value.split_whitespace();

        let cmd_str = tokens.next().ok_or("Empty command").unwrap();

        match cmd_str {
            "ping" => {
                let arg = tokens.next().unwrap_or("");
                Ok(Command::Ping {
                    payload: String::try_from(arg).map_err(|_| "Arg too long")?,
                })
            }
            "led" => {
                let arg = tokens.next().ok_or("Missing LED state")?;
                let on = arg == "on" || arg == "1";
                Ok(Command::Led { on })
            }
            "version" => Ok(Command::Version),
            _ => Err("Unknown command"),
        }
    }
}

pub fn reply(payload: &str, success: bool) -> String<64> {
    let mut ret = String::from_str(payload).unwrap();
    ret.push_str(if success { "\nOK\n" } else { "\nErr\n" })
        .unwrap();
    ret
}
