use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Dialect {
    Postgres,
    MySQL,
    SQLite,
}

impl Dialect {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Postgres => "postgres",
            Self::MySQL => "mysql",
            Self::SQLite => "sqlite",
        }
    }
}

impl std::str::FromStr for Dialect {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "postgres" | "postgresql" => Ok(Self::Postgres),
            "mysql" => Ok(Self::MySQL),
            "sqlite" | "sqlite3" => Ok(Self::SQLite),
            other => Err(format!("Unsupported database dialect: {}", other)),
        }
    }
}

impl TryFrom<String> for Dialect {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<Dialect> for String {
    fn from(value: Dialect) -> Self {
        value.as_str().to_string()
    }
}

impl std::fmt::Display for Dialect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
