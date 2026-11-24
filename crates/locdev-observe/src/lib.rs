//! The Debug Center's parsing: PHP's serialize format and VarDumper's data
//! (dumps), the stack/exception formats of each runtime (PHP, Laravel's log,
//! Node.js, Python, Rust), secret redaction, grouping and causes. No I/O:
//! the daemon feeds text in and stores what comes out.

pub mod dump;
pub mod finish;
pub mod laravel;
pub mod parse;
pub mod php;
pub mod redact;
