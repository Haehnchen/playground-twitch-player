#include "../src/twitch_chat.c"

static void test_extract_irc_tag_returns_decoded_value(void)
{
    g_autofree char *display_name = extract_irc_tag(
        "display-name=Alice\\sBob;color=#123456;emotes=25:0-4",
        "display-name"
    );
    g_autofree char *reply = extract_irc_tag(
        "reply-parent-msg-body=hello\\sworld\\:\\\\line\\r\\n;other=x",
        "reply-parent-msg-body"
    );

    g_assert_cmpstr(display_name, ==, "Alice Bob");
    g_assert_cmpstr(reply, ==, "hello world;\\line\r\n");
}

static void test_extract_irc_tag_matches_exact_key(void)
{
    g_autofree char *id = extract_irc_tag("room-id=123;id=456", "id");
    g_autofree char *missing = extract_irc_tag("display-name=;", "display-name");

    g_assert_cmpstr(id, ==, "456");
    g_assert_null(missing);
}

static void test_extract_sender_from_prefix(void)
{
    g_autofree char *sender = extract_sender_from_prefix(":alice!alice@example.test PRIVMSG #chan :hello");
    g_autofree char *fallback = extract_sender_from_prefix("PING :tmi.twitch.tv");

    g_assert_cmpstr(sender, ==, "alice");
    g_assert_cmpstr(fallback, ==, "chat");
}

static void test_parse_privmsg_with_tags(void)
{
    ParsedPrivmsg *message = parse_privmsg(
        "@display-name=Alice\\sBob;color=#123456;emotes=25:0-4;"
        "reply-parent-display-name=Carol;"
        "reply-parent-msg-body=previous\\smessage "
        ":alice!alice@example.test PRIVMSG #channel :Kappa hello\r"
    );

    g_assert_nonnull(message);
    g_assert_cmpstr(message->display_name, ==, "Alice Bob");
    g_assert_cmpstr(message->message, ==, "Kappa hello");
    g_assert_cmpstr(message->color, ==, "#123456");
    g_assert_cmpstr(message->emotes, ==, "25:0-4");
    g_assert_cmpstr(message->reply_display_name, ==, "Carol");
    g_assert_cmpstr(message->reply_message, ==, "previous message");

    parsed_privmsg_free(message);
}

static void test_parse_privmsg_falls_back_to_sender(void)
{
    ParsedPrivmsg *message = parse_privmsg(":fallback!user@example.test PRIVMSG #channel :hello");

    g_assert_nonnull(message);
    g_assert_cmpstr(message->display_name, ==, "fallback");
    g_assert_cmpstr(message->message, ==, "hello");
    g_assert_null(message->color);

    parsed_privmsg_free(message);
}

static void test_parse_privmsg_rejects_non_messages(void)
{
    g_assert_null(parse_privmsg("PING :tmi.twitch.tv"));
    g_assert_null(parse_privmsg(":server NOTICE #channel :hello"));
}

int main(int argc, char **argv)
{
    g_test_init(&argc, &argv, NULL);

    g_test_add_func("/twitch-chat/extract-irc-tag/decoded", test_extract_irc_tag_returns_decoded_value);
    g_test_add_func("/twitch-chat/extract-irc-tag/exact-key", test_extract_irc_tag_matches_exact_key);
    g_test_add_func("/twitch-chat/extract-sender", test_extract_sender_from_prefix);
    g_test_add_func("/twitch-chat/parse-privmsg/tags", test_parse_privmsg_with_tags);
    g_test_add_func("/twitch-chat/parse-privmsg/fallback-sender", test_parse_privmsg_falls_back_to_sender);
    g_test_add_func("/twitch-chat/parse-privmsg/rejects-non-messages", test_parse_privmsg_rejects_non_messages);

    return g_test_run();
}
