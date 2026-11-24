//! LocDev's Mailbox: a capture-only SMTP server on loopback, the messages it
//! keeps, and how each framework is pointed at it.

pub mod client;
pub mod parse;
pub mod smtp;
pub mod store;
pub mod wiring;

#[cfg(test)]
mod tests;
