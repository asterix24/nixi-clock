use core::str::FromStr;
use heapless::String;

fn hex_to_u8(input: &str) -> Option<u8> {
    let pulita = input.trim();
    let solo_hex = pulita
        .strip_prefix("0x")
        .or_else(|| pulita.strip_prefix("0X"))
        .unwrap_or(pulita);
    u8::from_str_radix(solo_hex, 16).ok()
}

#[derive(Debug)]
pub enum Command {
    Ping { payload: String<16> },
    Led { on: bool },
    Out { mask: u8 },
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
            "out" => {
                let arg = tokens.next().ok_or("Missing out bit mask")?;
                if let Some(mask) = hex_to_u8(arg) {
                    Ok(Command::Out { mask })
                } else {
                    Err("Invalid input")
                }
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
