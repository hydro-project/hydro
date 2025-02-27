pub mod membership;
pub mod model;

pub mod server;

pub mod lattices;

pub mod util;

use std::collections::HashSet;
use std::fmt::Display;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::KeyParseError::InvalidNamespace;
use crate::model::{Clock, Namespaces};

/// The namespace of the key of an entry in the key-value store.
#[derive(Debug, Eq, PartialEq, Clone, Copy, Serialize, Deserialize, Hash)]
pub enum Namespace {
    /// User namespace is for use by the user of the key-value store.
    User,

    /// System namespace is reserved for use by the key-value store itself.
    System,
}

/// Error that can occur when parsing a key from a string.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum KeyParseError {
    /// The namespace in the key is invalid. Namespaces must be either `usr` or `sys`.
    InvalidNamespace,

    /// The key is in an invalid format. Keys must be of the form `/namespace/table/row`.
    InvalidFormat,
}

impl Display for KeyParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvalidNamespace => write!(f, "Invalid namespace"),
            KeyParseError::InvalidFormat => write!(f, "Invalid key format"),
        }
    }
}

impl FromStr for Namespace {
    type Err = KeyParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "usr" => Ok(Namespace::User),
            "sys" => Ok(Namespace::System),
            _ => Err(InvalidNamespace),
        }
    }
}

/// The name of a table in the key-value store.
pub type TableName = String;

/// The key of a row in a table in the key-value store.
pub type RowKey = String;

/// A key of an entry in the key-value store.
///
/// Data in the key-value store is organized into namespaces, tables, and rows. Namespaces are
/// either `usr` for user data or `sys` for system data. Namespaces contain tables, which contain
/// rows. Each row has a row key and a row value.
#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize, Hash)]
pub struct Key {
    /// The namespace of the key.
    pub namespace: Namespace,
    /// The name of the table in the key.
    pub table: TableName,
    /// The key of the row in the table.
    pub row_key: RowKey,
}

impl FromStr for Key {
    type Err = KeyParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts: Vec<String> = vec![];
        let mut current_part = String::new();
        let mut escaping = false;

        for c in s.chars() {
            match (escaping, c) {
                (true, '\\') | (true, '/') => {
                    current_part.push(c);
                    escaping = false;
                }
                (true, _) => return Err(KeyParseError::InvalidFormat),
                (false, '\\') => {
                    escaping = true;
                }
                (false, '/') => {
                    parts.push(current_part);
                    current_part = String::new();
                }
                (false, _) => {
                    current_part.push(c);
                }
            }
        }

        if escaping {
            return Err(KeyParseError::InvalidFormat);
        }

        if !current_part.is_empty() {
            parts.push(current_part);
        }

        if parts.len() != 4 {
            return Err(KeyParseError::InvalidFormat);
        }

        if !parts[0].is_empty() {
            return Err(KeyParseError::InvalidFormat);
        }

        if parts[2].is_empty() || parts[3].is_empty() {
            return Err(KeyParseError::InvalidFormat);
        }

        let namespace = parts[1].parse()?;
        Ok(Key {
            namespace,
            table: parts[2].to_string(),
            row_key: parts[3].to_string(),
        })
    }
}

/// A request from a client to the key-value store.
#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub enum ClientRequest {
    /// A request to get the value of a key.
    Get { key: Key },
    /// A request to set the value of a key.
    Set { key: Key, value: String },
    /// A request to delete the value of a key.
    Delete { key: Key },
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub enum ClientResponse {
    /// A response for a get request. The key is echoed back along with the value, if it exists.
    /// Multiple values are returned if there were concurrent writes to the key.
    Get { key: Key, value: HashSet<String> },
    /// A response for a set request. The success field is true if the set was successful.
    Set { success: bool },
    /// A response for a delete request. The success field is true if delete was successful.
    Delete { success: bool },
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub enum GossipMessage {
    /// An "infecting message" to share updates with a peer.
    Gossip {
        message_id: String,
        member_id: String,
        writes: Namespaces<Clock>,
    },
    /// An acknowledgement message sent by a peer in response to a Gossip message, to indicate
    /// that it hasn't seen some of the writes in the Gossip message before.
    Ack {
        message_id: String,
        member_id: String,
    },
    /// A negative acknowledgement sent by a peer in response to a Gossip message, to indicate
    /// that it has seen all of the writes in the Gossip message before.
    Nack {
        message_id: String,
        member_id: String,
    },
}

#[cfg(test)]
mod tests {
    use super::{Key, Namespace};

    #[test]
    fn test_key_parsing_sys_namespace() {
        // Sys namespace
        let first = "/sys/sys_table/sys_row".parse::<Key>().unwrap();
        assert_eq!(first.namespace, Namespace::System);
        assert_eq!(first.table, "sys_table");
        assert_eq!(first.row_key, "sys_row");
    }
    #[test]
    fn test_key_parsing_user_namespace() {
        // User namespace
        let second = "/usr/usr_table/usr_row".parse::<Key>().unwrap();
        assert_eq!(second.namespace, Namespace::User);
        assert_eq!(second.table, "usr_table");
        assert_eq!(second.row_key, "usr_row");
    }

    #[test]
    fn test_key_empty_table() {
        // Empty table
        let empty_table = "/usr//usr_row".parse::<Key>();
        assert!(empty_table.is_err());
        assert_eq!(
            empty_table.unwrap_err(),
            super::KeyParseError::InvalidFormat
        );
    }

    #[test]
    fn test_key_empty_row() {
        // Empty row
        let empty_row = "/usr/usr_table/".parse::<Key>();
        assert!(empty_row.is_err());
        assert_eq!(empty_row.unwrap_err(), super::KeyParseError::InvalidFormat);
    }

    #[test]
    fn test_key_parsing_invalid_namespace() {
        // Invalid namespace
        let non_existent_namespace = "/ne_namespace/ne_table/ne_row".parse::<Key>();
        assert!(non_existent_namespace.is_err());
        assert_eq!(
            non_existent_namespace.unwrap_err(),
            super::KeyParseError::InvalidNamespace
        );
    }

    #[test]
    fn test_key_parsing_invalid_format() {
        // Invalid format
        let invalid_format = "/not_even_a_key".parse::<Key>();
        assert!(invalid_format.is_err());
        assert_eq!(
            invalid_format.unwrap_err(),
            super::KeyParseError::InvalidFormat
        );

        let invalid_format = "abcd/sys/sys_table/sys_row".parse::<Key>();
        assert!(invalid_format.is_err());
        assert_eq!(
            invalid_format.unwrap_err(),
            super::KeyParseError::InvalidFormat
        );
    }

    #[test]
    fn test_key_parsing_escaping() {
        // Escape \
        let key = r"/usr/usr\/table/usr\/row".parse::<Key>().unwrap();
        assert_eq!(key.namespace, Namespace::User);
        assert_eq!(key.table, r"usr/table");
        assert_eq!(key.row_key, r"usr/row");

        // Escaping /
        let key = r"/usr/usr\\table/usr\\row".parse::<Key>().unwrap();
        assert_eq!(key.namespace, Namespace::User);
        assert_eq!(key.table, r"usr\table");
        assert_eq!(key.row_key, r"usr\row");

        // Escaping any character
        let key = r"/usr/usr\table/usr\row".parse::<Key>();
        assert!(key.is_err());
        assert_eq!(key.unwrap_err(), super::KeyParseError::InvalidFormat);

        // Dangling escape
        let key = r"/usr/usr_table/usr_row\".parse::<Key>();
        assert!(key.is_err());
        assert_eq!(key.unwrap_err(), super::KeyParseError::InvalidFormat);
    }
}
