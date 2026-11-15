use super::GDestroyNotify;
use std::ffi::{c_char, c_int, c_uint, c_void};
pub(super) const G_FILE_TEST_EXISTS: c_int = 1 << 4;

#[repr(C)]
pub struct GPtrArray {
    pub(super) pdata: *mut *mut c_void,
    pub(super) len: c_uint,
}

#[repr(C)]
pub struct GError {
    pub domain: c_uint,
    pub code: c_int,
    pub message: *mut c_char,
}

#[repr(C)]
pub struct JsonParser {
    _private: [u8; 0],
}

#[repr(C)]
pub struct JsonNode {
    _private: [u8; 0],
}

#[repr(C)]
pub struct JsonObject {
    _private: [u8; 0],
}

#[repr(C)]
pub struct JsonArray {
    _private: [u8; 0],
}

#[repr(C)]
pub struct JsonBuilder {
    _private: [u8; 0],
}

#[repr(C)]
pub struct JsonGenerator {
    _private: [u8; 0],
}

unsafe extern "C" {
    pub(super) fn g_build_filenamev(args: *mut *mut c_char) -> *mut c_char;
    pub(super) fn g_clear_error(error: *mut *mut GError);
    pub(super) fn g_file_test(filename: *const c_char, test: c_int) -> c_int;
    pub(super) fn g_file_error_from_errno(err_no: c_int) -> c_int;
    pub(super) fn g_file_error_quark() -> c_uint;
    pub(super) fn g_free(mem: *mut c_void);
    pub(super) fn g_get_user_config_dir() -> *const c_char;
    pub(super) fn g_mkdir_with_parents(pathname: *const c_char, mode: c_int) -> c_int;
    pub(super) fn g_object_unref(object: *mut c_void);
    pub(super) fn g_ptr_array_add(array: *mut GPtrArray, data: *mut c_void);
    pub(super) fn g_ptr_array_new_with_free_func(
        element_free_func: Option<GDestroyNotify>,
    ) -> *mut GPtrArray;
    pub(super) fn g_ptr_array_set_size(array: *mut GPtrArray, length: c_uint);
    pub(super) fn g_ptr_array_unref(array: *mut GPtrArray);
    pub(super) fn g_set_error(
        error: *mut *mut GError,
        domain: c_uint,
        code: c_int,
        format: *const c_char,
        ...
    );
    pub(super) fn g_strdup(str: *const c_char) -> *mut c_char;
    pub(super) fn g_strerror(errnum: c_int) -> *const c_char;

    pub(super) fn json_array_get_element(array: *mut JsonArray, index_: c_uint) -> *mut JsonNode;
    pub(super) fn json_array_get_length(array: *mut JsonArray) -> c_uint;
    pub(super) fn json_builder_add_boolean_value(
        builder: *mut JsonBuilder,
        value: c_int,
    ) -> *mut JsonBuilder;
    pub(super) fn json_builder_add_int_value(
        builder: *mut JsonBuilder,
        value: i64,
    ) -> *mut JsonBuilder;
    pub(super) fn json_builder_add_string_value(
        builder: *mut JsonBuilder,
        value: *const c_char,
    ) -> *mut JsonBuilder;
    pub(super) fn json_builder_begin_array(builder: *mut JsonBuilder) -> *mut JsonBuilder;
    pub(super) fn json_builder_begin_object(builder: *mut JsonBuilder) -> *mut JsonBuilder;
    pub(super) fn json_builder_end_array(builder: *mut JsonBuilder) -> *mut JsonBuilder;
    pub(super) fn json_builder_end_object(builder: *mut JsonBuilder) -> *mut JsonBuilder;
    pub(super) fn json_builder_get_root(builder: *mut JsonBuilder) -> *mut JsonNode;
    pub(super) fn json_builder_new() -> *mut JsonBuilder;
    pub(super) fn json_builder_set_member_name(
        builder: *mut JsonBuilder,
        member_name: *const c_char,
    ) -> *mut JsonBuilder;
    pub(super) fn json_generator_new() -> *mut JsonGenerator;
    pub(super) fn json_generator_set_pretty(generator: *mut JsonGenerator, is_pretty: c_int);
    pub(super) fn json_generator_set_root(generator: *mut JsonGenerator, node: *mut JsonNode);
    pub(super) fn json_generator_to_file(
        generator: *mut JsonGenerator,
        filename: *const c_char,
        error: *mut *mut GError,
    ) -> c_int;
    pub(super) fn json_node_get_array(node: *mut JsonNode) -> *mut JsonArray;
    pub(super) fn json_node_get_node_type(node: *mut JsonNode) -> c_int;
    pub(super) fn json_node_get_object(node: *mut JsonNode) -> *mut JsonObject;
    pub(super) fn json_node_unref(node: *mut JsonNode);
    pub(super) fn json_object_get_boolean_member_with_default(
        object: *mut JsonObject,
        member_name: *const c_char,
        default_value: c_int,
    ) -> c_int;
    pub(super) fn json_object_get_int_member_with_default(
        object: *mut JsonObject,
        member_name: *const c_char,
        default_value: i64,
    ) -> i64;
    pub(super) fn json_object_get_member(
        object: *mut JsonObject,
        member_name: *const c_char,
    ) -> *mut JsonNode;
    pub(super) fn json_object_get_string_member_with_default(
        object: *mut JsonObject,
        member_name: *const c_char,
        default_value: *const c_char,
    ) -> *const c_char;
    pub(super) fn json_parser_get_root(parser: *mut JsonParser) -> *mut JsonNode;
    pub(super) fn json_parser_load_from_file(
        parser: *mut JsonParser,
        filename: *const c_char,
        error: *mut *mut GError,
    ) -> c_int;
    pub(super) fn json_parser_new() -> *mut JsonParser;
}
