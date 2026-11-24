//! A minimal RESP2 client for the key browser: enough for AUTH, SELECT,
//! SCAN, TYPE, TTL, reading each data type a page at a time, SET, EXPIRE
//! and DEL. Keys are listed with SCAN, never KEYS, so a large database
//! isn't blocked.

use std::time::Duration;

use locdev_core::db::{RedisKey, RedisValue};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

/// Items returned at most for one key.
const PAGE: usize = 200;

#[derive(Debug, Clone, PartialEq)]
pub enum Reply {
    Simple(String),
    Error(String),
    Int(i64),
    Bulk(Option<Vec<u8>>),
    Array(Option<Vec<Reply>>),
}

impl Reply {
    fn text(&self) -> Option<String> {
        match self {
            Self::Simple(s) => Some(s.clone()),
            Self::Bulk(Some(b)) => Some(String::from_utf8_lossy(b).into_owned()),
            Self::Int(i) => Some(i.to_string()),
            _ => None,
        }
    }

    fn items(self) -> Vec<Reply> {
        match self {
            Self::Array(Some(items)) => items,
            _ => Vec::new(),
        }
    }
}

pub struct Resp {
    stream: BufReader<TcpStream>,
}

impl Resp {
    pub async fn connect(
        host: &str,
        port: u16,
        user: Option<&str>,
        password: Option<&str>,
        db: u32,
    ) -> Result<Self, String> {
        let stream = tokio::time::timeout(Duration::from_secs(5), TcpStream::connect((host, port)))
            .await
            .map_err(|_| "timed out connecting".to_string())?
            .map_err(|e| e.to_string())?;
        let mut resp = Self {
            stream: BufReader::new(stream),
        };
        if let Some(password) = password {
            let reply = match user {
                Some(user) => resp.call(&["AUTH", user, password]).await?,
                None => resp.call(&["AUTH", password]).await?,
            };
            if let Reply::Error(e) = reply {
                return Err(e);
            }
        }
        if db != 0 {
            resp.ok(&["SELECT", &db.to_string()]).await?;
        }
        Ok(resp)
    }

    pub async fn call(&mut self, args: &[&str]) -> Result<Reply, String> {
        let mut out = format!("*{}\r\n", args.len()).into_bytes();
        for a in args {
            out.extend(format!("${}\r\n", a.len()).as_bytes());
            out.extend(a.as_bytes());
            out.extend(b"\r\n");
        }
        tokio::time::timeout(Duration::from_secs(30), async {
            self.stream
                .get_mut()
                .write_all(&out)
                .await
                .map_err(|e| e.to_string())?;
            read(&mut self.stream).await
        })
        .await
        .map_err(|_| "timed out".to_string())?
    }

    async fn ok(&mut self, args: &[&str]) -> Result<Reply, String> {
        match self.call(args).await? {
            Reply::Error(e) => Err(e),
            other => Ok(other),
        }
    }

    pub async fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        // KEEPTTL: changing a value doesn't drop its expiry.
        self.ok(&["SET", key, value, "KEEPTTL"]).await.map(|_| ())
    }

    pub async fn expire(&mut self, key: &str, seconds: Option<i64>) -> Result<(), String> {
        match seconds {
            Some(s) => self.ok(&["EXPIRE", key, &s.to_string()]).await,
            None => self.ok(&["PERSIST", key]).await,
        }
        .map(|_| ())
    }

    pub async fn delete(&mut self, key: &str) -> Result<i64, String> {
        match self.ok(&["DEL", key]).await? {
            Reply::Int(n) => Ok(n),
            _ => Ok(0),
        }
    }
}

fn read(
    stream: &mut BufReader<TcpStream>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Reply, String>> + Send + '_>> {
    Box::pin(async move {
        let mut line = String::new();
        stream
            .read_line(&mut line)
            .await
            .map_err(|e| e.to_string())?;
        let line = line.trim_end_matches("\r\n");
        let (kind, rest) = line.split_at(line.len().min(1));
        Ok(match kind {
            "+" => Reply::Simple(rest.into()),
            "-" => Reply::Error(rest.into()),
            ":" => Reply::Int(rest.parse().map_err(|_| "bad integer")?),
            "$" => {
                let len: i64 = rest.parse().map_err(|_| "bad length")?;
                if len < 0 {
                    Reply::Bulk(None)
                } else if len > 64 * 1024 * 1024 {
                    return Err("value larger than 64 MB".into());
                } else {
                    let mut buf = vec![0u8; len as usize + 2];
                    stream
                        .read_exact(&mut buf)
                        .await
                        .map_err(|e| e.to_string())?;
                    buf.truncate(len as usize);
                    Reply::Bulk(Some(buf))
                }
            }
            "*" => {
                let len: i64 = rest.parse().map_err(|_| "bad length")?;
                if len < 0 {
                    Reply::Array(None)
                } else {
                    let mut items = Vec::with_capacity(len.min(10_000) as usize);
                    for _ in 0..len {
                        items.push(read(stream).await?);
                    }
                    Reply::Array(Some(items))
                }
            }
            other => return Err(format!("unexpected reply {other:?}")),
        })
    })
}

/// One SCAN step: the next cursor ("0" when done) and the keys with type and TTL.
pub async fn redis_scan(
    r: &mut Resp,
    cursor: &str,
    pattern: &str,
) -> Result<(String, Vec<RedisKey>), String> {
    let reply = r
        .ok(&["SCAN", cursor, "MATCH", pattern, "COUNT", "200"])
        .await?;
    let mut parts = reply.items().into_iter();
    let next = parts
        .next()
        .and_then(|c| c.text())
        .unwrap_or_else(|| "0".into());
    let keys: Vec<String> = parts
        .next()
        .map(Reply::items)
        .unwrap_or_default()
        .iter()
        .filter_map(Reply::text)
        .collect();
    let mut out = Vec::with_capacity(keys.len());
    for key in keys {
        let kind = r.ok(&["TYPE", &key]).await?.text().unwrap_or_default();
        let ttl = match r.ok(&["TTL", &key]).await? {
            Reply::Int(t) => t,
            _ => -1,
        };
        out.push(RedisKey { key, kind, ttl });
    }
    Ok((next, out))
}

/// A key's value, a page at a time.
pub async fn redis_get(r: &mut Resp, key: &str) -> Result<RedisValue, String> {
    let kind = r.ok(&["TYPE", key]).await?.text().unwrap_or_default();
    let ttl = match r.ok(&["TTL", key]).await? {
        Reply::Int(t) => t,
        _ => -1,
    };
    let last = (PAGE - 1).to_string();
    let single = |items: Vec<Reply>| -> Vec<(String, Option<String>)> {
        items
            .iter()
            .filter_map(Reply::text)
            .map(|v| (v, None))
            .collect()
    };
    let pairs = |items: Vec<Reply>| -> Vec<(String, Option<String>)> {
        items
            .chunks(2)
            .filter_map(|c| Some((c.first()?.text()?, c.get(1).and_then(Reply::text))))
            .collect()
    };
    let (items, total) = match kind.as_str() {
        "string" => {
            let value = r.ok(&["GET", key]).await?.text();
            (vec![(value.unwrap_or_default(), None)], 1)
        }
        "list" => {
            let total = r
                .ok(&["LLEN", key])
                .await?
                .text()
                .and_then(|t| t.parse().ok())
                .unwrap_or(0usize);
            (
                single(r.ok(&["LRANGE", key, "0", &last]).await?.items()),
                total,
            )
        }
        "hash" => {
            let total = r
                .ok(&["HLEN", key])
                .await?
                .text()
                .and_then(|t| t.parse().ok())
                .unwrap_or(0usize);
            let reply = r
                .ok(&["HSCAN", key, "0", "COUNT", &PAGE.to_string()])
                .await?;
            let items = reply
                .items()
                .into_iter()
                .nth(1)
                .map(Reply::items)
                .unwrap_or_default();
            (pairs(items), total)
        }
        "set" => {
            let total = r
                .ok(&["SCARD", key])
                .await?
                .text()
                .and_then(|t| t.parse().ok())
                .unwrap_or(0usize);
            let reply = r
                .ok(&["SSCAN", key, "0", "COUNT", &PAGE.to_string()])
                .await?;
            (
                single(
                    reply
                        .items()
                        .into_iter()
                        .nth(1)
                        .map(Reply::items)
                        .unwrap_or_default(),
                ),
                total,
            )
        }
        "zset" => {
            let total = r
                .ok(&["ZCARD", key])
                .await?
                .text()
                .and_then(|t| t.parse().ok())
                .unwrap_or(0usize);
            (
                pairs(
                    r.ok(&["ZRANGE", key, "0", &last, "WITHSCORES"])
                        .await?
                        .items(),
                ),
                total,
            )
        }
        "none" => return Err(format!("no key {key}")),
        // Streams and module types: shown by type only.
        _ => (Vec::new(), 0),
    };
    Ok(RedisValue {
        key: key.into(),
        kind,
        ttl,
        truncated: total > items.len(),
        items,
    })
}
