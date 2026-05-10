use std::ffi::{c_char, c_void, CStr, CString};
use twitch_player_core::twitch_chat;

unsafe extern "C" {
    fn g_free(mem: *mut c_void);
}

fn cstring(value: &str) -> CString {
    CString::new(value).unwrap()
}

unsafe fn assert_str_eq(actual: *const c_char, expected: &str) {
    assert!(!actual.is_null());
    assert_eq!(CStr::from_ptr(actual).to_str().unwrap(), expected);
}

unsafe fn extract_irc_tag(tags: &str, key: &str) -> *mut c_char {
    twitch_chat::twitch_chat_test_extract_irc_tag(cstring(tags).as_ptr(), cstring(key).as_ptr())
}

unsafe fn extract_sender_from_prefix(line: &str) -> *mut c_char {
    twitch_chat::twitch_chat_test_extract_sender_from_prefix(cstring(line).as_ptr())
}

unsafe fn parse_privmsg(line: &str) -> *mut twitch_chat::ParsedPrivmsg {
    twitch_chat::twitch_chat_test_parse_privmsg(cstring(line).as_ptr())
}

unsafe fn test_extract_irc_tag_returns_decoded_value() {
    let display_name = extract_irc_tag(
        "display-name=Alice\\sBob;color=#123456;emotes=25:0-4",
        "display-name",
    );
    let reply = extract_irc_tag(
        "reply-parent-msg-body=hello\\sworld\\:\\\\line\\r\\n;other=x",
        "reply-parent-msg-body",
    );

    assert_str_eq(display_name, "Alice Bob");
    assert_str_eq(reply, "hello world;\\line\r\n");
    g_free(display_name as *mut c_void);
    g_free(reply as *mut c_void);
}

unsafe fn test_extract_irc_tag_matches_exact_key() {
    let id = extract_irc_tag("room-id=123;id=456", "id");
    let missing = extract_irc_tag("display-name=;", "display-name");

    assert_str_eq(id, "456");
    assert!(missing.is_null());
    g_free(id as *mut c_void);
}

unsafe fn test_extract_sender_from_prefix() {
    let sender = extract_sender_from_prefix(":alice!alice@example.test PRIVMSG #chan :hello");
    let fallback = extract_sender_from_prefix("PING :tmi.twitch.tv");

    assert_str_eq(sender, "alice");
    assert_str_eq(fallback, "chat");
    g_free(sender as *mut c_void);
    g_free(fallback as *mut c_void);
}

unsafe fn test_parse_privmsg_with_tags() {
    let message = parse_privmsg(
        "@display-name=Alice\\sBob;color=#123456;emotes=25:0-4;\
         reply-parent-display-name=Carol;\
         reply-parent-msg-body=previous\\smessage \
         :alice!alice@example.test PRIVMSG #channel :Kappa hello\r",
    );

    assert!(!message.is_null());
    assert_str_eq((*message).display_name, "Alice Bob");
    assert_str_eq((*message).message, "Kappa hello");
    assert_str_eq((*message).color, "#123456");
    assert_str_eq((*message).emotes, "25:0-4");
    assert_str_eq((*message).reply_display_name, "Carol");
    assert_str_eq((*message).reply_message, "previous message");

    twitch_chat::twitch_chat_test_parsed_privmsg_free(message);
}

unsafe fn test_parse_privmsg_falls_back_to_sender() {
    let message = parse_privmsg(":fallback!user@example.test PRIVMSG #channel :hello");

    assert!(!message.is_null());
    assert_str_eq((*message).display_name, "fallback");
    assert_str_eq((*message).message, "hello");
    assert!((*message).color.is_null());

    twitch_chat::twitch_chat_test_parsed_privmsg_free(message);
}

unsafe fn test_parse_privmsg_rejects_non_messages() {
    assert!(parse_privmsg("PING :tmi.twitch.tv").is_null());
    assert!(parse_privmsg(":server NOTICE #channel :hello").is_null());
}

fn main() {
    unsafe {
        test_extract_irc_tag_returns_decoded_value();
        test_extract_irc_tag_matches_exact_key();
        test_extract_sender_from_prefix();
        test_parse_privmsg_with_tags();
        test_parse_privmsg_falls_back_to_sender();
        test_parse_privmsg_rejects_non_messages();
    }
}
