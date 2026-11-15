use super::{GDestroyNotify, GSourceFunc, GThreadFunc};
use std::ffi::{c_char, c_int, c_uint, c_void};
pub(super) const G_IO_ERROR_CANCELLED: c_int = 19;
pub(super) const G_SOURCE_REMOVE: c_int = 0;

pub use gtk4::gio::ffi::{
    GCancellable, GDataInputStream, GIOStream, GInputStream, GOutputStream, GSocket, GSocketClient,
    GSocketConnection,
};

pub use gtk4::glib::ffi::{GError, GThread};

unsafe extern "C" {
    pub(super) fn g_cancellable_cancel(cancellable: *mut GCancellable);
    pub(super) fn g_cancellable_is_cancelled(cancellable: *mut GCancellable) -> c_int;
    pub(super) fn g_cancellable_new() -> *mut GCancellable;
    pub(super) fn g_clear_error(error: *mut *mut GError);
    pub(super) fn g_data_input_stream_new(base_stream: *mut GInputStream) -> *mut GDataInputStream;
    pub(super) fn g_data_input_stream_read_line_utf8(
        stream: *mut GDataInputStream,
        length: *mut usize,
        cancellable: *mut GCancellable,
        error: *mut *mut GError,
    ) -> *mut c_char;
    pub(super) fn g_error_matches(error: *const GError, domain: c_uint, code: c_int) -> c_int;
    pub(super) fn g_free(mem: *mut c_void);
    pub(super) fn g_io_error_quark() -> c_uint;
    pub(super) fn g_io_stream_get_input_stream(stream: *mut GIOStream) -> *mut GInputStream;
    pub(super) fn g_io_stream_get_output_stream(stream: *mut GIOStream) -> *mut GOutputStream;
    pub(super) fn g_main_context_invoke_full(
        context: *mut c_void,
        priority: c_int,
        function: Option<GSourceFunc>,
        data: *mut c_void,
        notify: Option<GDestroyNotify>,
    );
    pub(super) fn g_object_ref(object: *mut c_void) -> *mut c_void;
    pub(super) fn g_object_unref(object: *mut c_void);
    pub(super) fn g_output_stream_write_all(
        stream: *mut GOutputStream,
        buffer: *const c_void,
        count: usize,
        bytes_written: *mut usize,
        cancellable: *mut GCancellable,
        error: *mut *mut GError,
    ) -> c_int;
    pub(super) fn g_random_int_range(begin: c_int, end: c_int) -> c_int;
    pub(super) fn g_socket_client_connect_to_host(
        client: *mut GSocketClient,
        host_and_port: *const c_char,
        default_port: u16,
        cancellable: *mut GCancellable,
        error: *mut *mut GError,
    ) -> *mut GSocketConnection;
    pub(super) fn g_socket_client_new() -> *mut GSocketClient;
    pub(super) fn g_socket_client_set_timeout(client: *mut GSocketClient, timeout: c_uint);
    pub(super) fn g_socket_client_set_tls(client: *mut GSocketClient, tls: c_int);
    pub(super) fn g_socket_connection_get_socket(
        connection: *mut GSocketConnection,
    ) -> *mut GSocket;
    pub(super) fn g_socket_set_timeout(socket: *mut GSocket, timeout: c_uint);
    pub(super) fn g_thread_new(
        name: *const c_char,
        func: Option<GThreadFunc>,
        data: *mut c_void,
    ) -> *mut GThread;
    pub(super) fn g_thread_unref(thread: *mut GThread);
    pub(super) fn g_usleep(microseconds: u64);
}
