//! The launcher's capability boundary: game control commands cannot pass here.
#[derive(Debug, PartialEq)]
pub(super) enum Command<'a> {
    Hello,
    Friends(&'a str),
    Join(&'a str),
}
pub(super) fn parse(line: &str) -> Result<Command<'_>, &'static str> {
    let words: Vec<_> = line.split_whitespace().collect();
    match words.as_slice() {
        ["hello"] => Ok(Command::Hello),
        [action @ ("host" | "stop" | "status")] => Ok(Command::Friends(action)),
        ["join", address]
            if address.strip_prefix("friend:").is_some_and(|code| {
                (8..=1000).contains(&code.len())
                    && code.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            }) =>
        {
            Ok(Command::Join(address))
        }
        _ => Err("VIS Portals accepts hello|host|stop|status|join friend:CODE"),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn friends_commands_cannot_become_game_control_or_arbitrary_connections() {
        assert_eq!(parse("hello"), Ok(Command::Hello));
        assert_eq!(parse("host"), Ok(Command::Friends("host")));
        assert_eq!(
            parse("join friend:ABCDE-FGHIJ"),
            Ok(Command::Join("friend:ABCDE-FGHIJ"))
        );
        for command in [
            "quit",
            "give diamond 64",
            "connect localhost:25565",
            "join localhost:25565",
            "join friend:ABCDE-FGHIJ\nquit",
            "host extra",
        ] {
            assert!(parse(command).is_err(), "{command}");
        }
    }
}
