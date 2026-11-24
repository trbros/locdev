//! The database manager as clients see it. Clients only ever hold a
//! connection id: hosts, users and passwords are resolved by the daemon
//! each time, from the project's files or a LocDev instance.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DbEngine {
    Postgres,
    Mysql,
    Sqlite,
    Redis,
}

impl DbEngine {
    pub fn label(self) -> &'static str {
        match self {
            Self::Postgres => "PostgreSQL",
            Self::Mysql => "MySQL/MariaDB",
            Self::Sqlite => "SQLite",
            Self::Redis => "Redis/Valkey",
        }
    }

    pub fn is_sql(self) -> bool {
        !matches!(self, Self::Redis)
    }
}

/// A database a project or an instance points at. No secret in here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbConnection {
    /// `instance:<name>` or `project:<site>:<n>`; all a client sends back.
    pub id: String,
    pub label: String,
    pub engine: DbEngine,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub database: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    /// A password is set (never sent).
    pub has_password: bool,
    /// Where it was found: `.env (DB_*)`, `.env (DATABASE_URL)`, `LocDev instance`…
    pub source: String,
    #[serde(default)]
    pub project: Option<String>,
    /// Not on this machine: opening it needs an explicit choice.
    pub remote: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbTable {
    pub name: String,
    /// Postgres schema (`public`); empty elsewhere.
    #[serde(default)]
    pub schema: String,
    /// `table` or `view`.
    pub kind: String,
    #[serde(default)]
    pub rows_estimate: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbColumn {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    #[serde(default)]
    pub default: Option<String>,
    pub primary_key: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbIndex {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbForeignKey {
    pub name: String,
    pub columns: Vec<String>,
    pub references_table: String,
    pub references_columns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbTableInfo {
    pub table: DbTable,
    pub columns: Vec<DbColumn>,
    pub indexes: Vec<DbIndex>,
    pub foreign_keys: Vec<DbForeignKey>,
}

/// One filter on a column (`= 'x'`, `IS NULL`…).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbFilter {
    pub column: String,
    /// `=`, `!=`, `<`, `>`, `<=`, `>=`, `like`, `is_null`, `not_null`.
    pub op: String,
    #[serde(default)]
    pub value: Option<String>,
}

/// Rows as text (`None` is NULL), at most a page.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DbRows {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
    /// Rows matching (for a table page), when counted.
    #[serde(default)]
    pub total: Option<u64>,
    /// Rows changed by a statement that returns none.
    #[serde(default)]
    pub affected: Option<u64>,
    /// More rows existed than were returned.
    pub truncated: bool,
    pub elapsed_ms: u64,
}

/// What a statement would do, judged before running it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct QueryCheck {
    /// Statements that drop, truncate or change every row.
    pub destructive: Vec<String>,
    /// Changes data or schema (not a plain read).
    pub writes: bool,
    /// Text to type back before a destructive statement runs (the database's name).
    #[serde(default)]
    pub confirm: Option<String>,
}

/// A Redis key with its type and time to live.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedisKey {
    pub key: String,
    /// string, list, hash, set, zset, stream…
    pub kind: String,
    /// Seconds; -1 = no expiry.
    pub ttl: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedisValue {
    pub key: String,
    pub kind: String,
    pub ttl: i64,
    /// Strings: the value; lists/sets: members; hashes: field/value pairs;
    /// sorted sets: member/score pairs. Capped at a page.
    pub items: Vec<(String, Option<String>)>,
    pub truncated: bool,
}

/// A change to one row, by primary key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum RowEdit {
    Update {
        key: Vec<(String, Option<String>)>,
        changes: Vec<(String, Option<String>)>,
    },
    Insert {
        values: Vec<(String, Option<String>)>,
    },
    Delete {
        key: Vec<(String, Option<String>)>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum RedisEdit {
    Set {
        key: String,
        value: String,
    },
    /// `None` removes the expiry.
    Expire {
        key: String,
        seconds: Option<i64>,
    },
    Delete {
        key: String,
    },
}
