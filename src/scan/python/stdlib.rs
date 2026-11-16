//! The python driver's STATIC standard-library name list (python driver
//! record P3 / decision-check C1-C2): the fixed union of the CPython 3.9
//! through 3.13 top-level module names — no version modeling (P3's own law).
//! Generated from CPython 3.14's `sys.stdlib_module_names` plus the names
//! later releases dropped inside the window (`distutils`, `imp`, `formatter`,
//! `asynchat`, `asyncore`, `telnetlib`, `nntplib`, `smtpd`-era companions and
//! the 3.13 removals below). A list miss can only drift the attribution
//! label, never the verdict: an unlisted stdlib name stays an EXTERNAL fact
//! (the C2 law), so this table needs no regeneration to keep a verdict true.
//! The ts R7 builtins list is the cited shape; the go no-dot rule does NOT
//! port — PyPI import names carry no dot, so dottedness carries no signal.

/// True when the top-level name is a stdlib name in the 3.9-3.13 union.
/// Own-package resolution runs BEFORE this table (the P3 tier order), so a
/// package-root directory named like a stdlib module claims first-party.
/// The driver NEVER branches on this table into different bytes — the P3
/// law says a list miss can only drift the attribution label, never the
/// verdict, so no legal code path exists that a stdlib hit could take
/// differently. The named consumers are the shape guards below; shipping
/// the data with an honest `allow` beats a fake consumer that pretends the
/// table gates something.
#[allow(dead_code)]
pub(crate) const STDLIB_NAMES: &[&str] = &[
    "__future__", "_abc", "_aix_support", "_android_support", "_apple_support", "_ast",
    "_ast_unparse", "_asyncio", "_bisect", "_blake2", "_bz2", "_codecs", "_codecs_cn", "_codecs_hk",
    "_codecs_iso2022", "_codecs_jp", "_codecs_kr", "_codecs_tw", "_collections", "_collections_abc",
    "_colorize", "_compat_pickle", "_contextvars", "_csv", "_ctypes", "_curses", "_curses_panel",
    "_datetime", "_dbm", "_decimal", "_elementtree", "_frozen_importlib",
    "_frozen_importlib_external", "_functools", "_gdbm", "_hashlib", "_heapq", "_hmac", "_imp",
    "_interpchannels", "_interpqueues", "_interpreters", "_io", "_ios_support", "_json", "_locale",
    "_lsprof", "_lzma", "_markupbase", "_md5", "_multibytecodec", "_multiprocessing", "_opcode",
    "_opcode_metadata", "_operator", "_osx_support", "_overlapped", "_pickle", "_posixshmem",
    "_posixsubprocess", "_py_abc", "_py_warnings", "_pydatetime", "_pydecimal", "_pyio", "_pylong",
    "_pyrepl", "_queue", "_random", "_remote_debugging", "_scproxy", "_sha1", "_sha2", "_sha3",
    "_signal", "_sitebuiltins", "_socket", "_sqlite3", "_sre", "_ssl", "_stat", "_statistics",
    "_string", "_strptime", "_struct", "_suggestions", "_symtable", "_sysconfig", "_thread",
    "_threading_local", "_tkinter", "_tokenize", "_tracemalloc", "_types", "_typing", "_uuid",
    "_warnings", "_weakref", "_weakrefset", "_winapi", "_wmi", "_zoneinfo", "_zstd", "abc", "aifc",
    "annotationlib", "antigravity", "argparse", "array", "ast", "asynchat", "asyncio", "asyncore",
    "atexit", "audioop", "base64", "bdb", "binascii", "binhex", "bisect", "builtins", "bz2",
    "cProfile", "calendar", "chunk", "cmath", "cmd", "code", "codecs", "codeop", "collections",
    "colorsys", "compileall", "compression", "concurrent", "configparser", "contextlib",
    "contextvars", "copy", "copyreg", "crypt", "csv", "ctypes", "curses", "dataclasses", "datetime",
    "dbm", "decimal", "difflib", "dis", "distutils", "doctest", "email", "encodings", "ensurepip",
    "enum", "errno", "faulthandler", "fcntl", "filecmp", "fileinput", "fnmatch", "formatter",
    "fractions", "ftplib", "functools", "gc", "genericpath", "getopt", "getpass", "gettext", "glob",
    "graphlib", "grp", "gzip", "hashlib", "heapq", "hmac", "html", "http", "idlelib", "imaplib",
    "imp", "importlib", "inspect", "io", "ipaddress", "itertools", "json", "keyword", "linecache",
    "locale", "logging", "lzma", "macpath", "mailbox", "marshal", "math", "mimetypes", "mmap",
    "modulefinder", "msilib", "msvcrt", "multiprocessing", "netrc", "nntplib", "nt", "ntpath",
    "nturl2path", "numbers", "opcode", "operator", "optparse", "os", "pathlib", "pdb", "pickle",
    "pickletools", "pipes", "pkgutil", "platform", "plistlib", "poplib", "posix", "posixpath",
    "pprint", "profile", "pstats", "pty", "pwd", "py_compile", "pyclbr", "pydoc", "pydoc_data",
    "pyexpat", "queue", "quopri", "random", "re", "readline", "reprlib", "resource", "rlcompleter",
    "runpy", "sched", "secrets", "select", "selectors", "shelve", "shlex", "shutil", "signal",
    "site", "smtplib", "sndhdr", "socket", "socketserver", "spwd", "sqlite3", "sre_compile",
    "sre_constants", "sre_parse", "ssl", "stat", "statistics", "string", "stringprep", "struct",
    "subprocess", "sunau", "symbol", "symtable", "sys", "sysconfig", "syslog", "tabnanny",
    "tarfile", "telnetlib", "tempfile", "termios", "textwrap", "this", "threading", "time",
    "timeit", "tkinter", "token", "tokenize", "tomllib", "trace", "traceback", "tracemalloc", "tty",
    "turtle", "turtledemo", "types", "typing", "unicodedata", "unittest", "urllib", "uu", "uuid",
    "venv", "warnings", "wave", "weakref", "webbrowser", "winreg", "winsound", "wsgiref", "xdrlib",
    "xml", "xmlrpc", "zipapp", "zipfile", "zipimport", "zlib", "zoneinfo",
];

#[allow(dead_code)]
pub(crate) fn is_stdlib(name: &str) -> bool {
    STDLIB_NAMES.binary_search(&name).is_ok()
}

#[cfg(test)]
mod tests {
    use super::is_stdlib;

    #[test]
    fn the_window_shapes_resolve_and_the_backport_direction_stays_external() {
        for name in ["os", "typing", "logging", "urllib", "asyncio", "re", "uuid"] {
            assert!(is_stdlib(name), "{name} is stdlib at every version in the window");
        }
        // The 3.9-12-era names later removals dropped, kept per the C2 law.
        for name in ["distutils", "imp", "asynchat", "asyncore", "formatter", "telnetlib"] {
            assert!(is_stdlib(name), "{name} was stdlib inside the window");
        }
        // The tomllib-backport direction: a PyPI backport name is external.
        assert!(!is_stdlib("typing_extensions"));
        assert!(!is_stdlib("sqlalchemy"));
        assert!(!is_stdlib("pytest"));
    }

    #[test]
    fn the_table_is_sorted_for_binary_search() {
        let mut sorted = super::STDLIB_NAMES.to_vec();
        sorted.sort_unstable();
        assert_eq!(sorted, super::STDLIB_NAMES, "is_stdlib binary-searches the table");
    }
}
