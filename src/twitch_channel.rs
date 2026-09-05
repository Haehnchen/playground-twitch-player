pub fn normalize_twitch_channel(value: &[u8]) -> Option<Vec<u8>> {
    let value = trim_ascii(value);
    if value.is_empty() {
        return None;
    }

    let lowercase: Vec<u8> = value.iter().map(u8::to_ascii_lowercase).collect();
    let mut candidate = lowercase.as_slice();
    let mut from_url = false;

    if let Some(without_scheme) = candidate
        .strip_prefix(b"https://")
        .or_else(|| candidate.strip_prefix(b"http://"))
    {
        candidate = without_scheme;
        from_url = true;
    }

    if let Some(without_host) = candidate
        .strip_prefix(b"www.twitch.tv/")
        .or_else(|| candidate.strip_prefix(b"m.twitch.tv/"))
        .or_else(|| candidate.strip_prefix(b"twitch.tv/"))
    {
        candidate = without_host;
        from_url = true;
    } else if from_url {
        return None;
    }

    while matches!(candidate.first(), Some(b'/' | b'@')) {
        candidate = &candidate[1..];
    }

    let channel_len = candidate
        .iter()
        .take_while(|byte| byte.is_ascii_alphanumeric() || **byte == b'_')
        .count();
    if channel_len == 0 || (!from_url && channel_len != candidate.len()) {
        return None;
    }

    Some(candidate[..channel_len].to_vec())
}

fn trim_ascii(mut value: &[u8]) -> &[u8] {
    while value.first().is_some_and(u8::is_ascii_whitespace) {
        value = &value[1..];
    }
    while value.last().is_some_and(u8::is_ascii_whitespace) {
        value = &value[..value.len() - 1];
    }
    value
}

#[cfg(test)]
mod tests {
    use super::normalize_twitch_channel;

    #[test]
    fn normalizes_channel_names_and_urls() {
        assert_eq!(
            normalize_twitch_channel(b"PapaPlatte"),
            Some(b"papaplatte".to_vec())
        );
        assert_eq!(
            normalize_twitch_channel(b"  @Some_Channel  "),
            Some(b"some_channel".to_vec())
        );
        assert_eq!(
            normalize_twitch_channel(b"https://www.twitch.tv/PapaPlatte"),
            Some(b"papaplatte".to_vec())
        );
        assert_eq!(
            normalize_twitch_channel(b"www.twitch.tv/PapaPlatte"),
            Some(b"papaplatte".to_vec())
        );
        assert_eq!(
            normalize_twitch_channel(b"HTTP://M.TWITCH.TV/Some_Channel/videos"),
            Some(b"some_channel".to_vec())
        );
    }

    #[test]
    fn rejects_empty_or_invalid_channels() {
        assert_eq!(normalize_twitch_channel(b""), None);
        assert_eq!(normalize_twitch_channel(b"   "), None);
        assert_eq!(normalize_twitch_channel(b"channel name"), None);
        assert_eq!(normalize_twitch_channel(b"channel!"), None);
        assert_eq!(normalize_twitch_channel(b"www.channel"), None);
        assert_eq!(
            normalize_twitch_channel(b"https://example.com/channel"),
            None
        );
        assert_eq!(normalize_twitch_channel(b"notwitch.tv/channel"), None);
    }
}
