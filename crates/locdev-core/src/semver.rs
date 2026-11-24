//! Versions and version constraints as npm and Composer write them.
//!
//! Both ecosystems share the shape (an OR of AND-ed comparators) but not the
//! sugar: `~1.2` means `<1.3` to npm and `<2.0` to Composer. Anything that is
//! not a version range (tags, URLs, `workspace:*`, `dev-main`) fails to parse,
//! so callers report it as "unknown" instead of guessing.

use std::cmp::Ordering;
use std::fmt;

/// A version with up to four numeric parts (Composer allows a fourth) and an
/// optional pre-release.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Version {
    pub nums: [u64; 4],
    pub pre: Vec<Ident>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ident {
    Num(u64),
    Text(String),
}

impl Ord for Ident {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Num(a), Self::Num(b)) => a.cmp(b),
            // SemVer: numeric identifiers sort before alphanumeric ones.
            (Self::Num(_), Self::Text(_)) => Ordering::Less,
            (Self::Text(_), Self::Num(_)) => Ordering::Greater,
            (Self::Text(a), Self::Text(b)) => a.cmp(b),
        }
    }
}

impl PartialOrd for Ident {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.nums.cmp(&other.nums).then_with(|| {
            match (self.pre.is_empty(), other.pre.is_empty()) {
                (true, true) => Ordering::Equal,
                // A release sorts after its pre-releases.
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => self.pre.cmp(&other.pre),
            }
        })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [a, b, c, d] = self.nums;
        write!(f, "{a}.{b}.{c}")?;
        if d > 0 {
            write!(f, ".{d}")?;
        }
        for (i, ident) in self.pre.iter().enumerate() {
            f.write_str(if i == 0 { "-" } else { "." })?;
            match ident {
                Ident::Num(n) => write!(f, "{n}")?,
                Ident::Text(t) => f.write_str(t)?,
            }
        }
        Ok(())
    }
}

impl Version {
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            nums: [major, minor, patch, 0],
            pre: Vec::new(),
        }
    }

    pub fn major(&self) -> u64 {
        self.nums[0]
    }

    pub fn minor(&self) -> u64 {
        self.nums[1]
    }

    pub fn is_prerelease(&self) -> bool {
        !self.pre.is_empty()
    }

    /// Parses `1.2.3`, `v1.2.3-beta.1+build`; missing parts count as 0.
    pub fn parse(raw: &str) -> Option<Self> {
        let partial = Partial::parse(raw.trim())?;
        (!partial.wild).then(|| partial.floor())
    }

    /// Like [`Version::parse`], plus Composer's stability suffixes
    /// (`1.0.0-beta2`, `2.1-RC1`, `3.0.0-dev`), mapped so they order
    /// dev < alpha < beta < RC < stable.
    pub fn parse_composer(raw: &str) -> Option<Self> {
        let raw = raw.trim().trim_start_matches(['v', 'V']);
        let (core, suffix) = match raw.find(['-', '+']) {
            Some(i) => (&raw[..i], Some(raw[i + 1..].to_ascii_lowercase())),
            None => (raw, None),
        };
        let mut version = Self::parse(core)?;
        if let Some(suffix) = suffix {
            let suffix = suffix.trim_start_matches(['.', '-']);
            let (word, rest) = suffix.split_at(
                suffix
                    .find(|c: char| c.is_ascii_digit() || c == '.')
                    .unwrap_or(suffix.len()),
            );
            let rank = match word {
                "dev" => 0,
                "alpha" | "a" => 1,
                "beta" | "b" => 2,
                "rc" => 3,
                // `-p1`/`-pl1` are patch releases: not pre-releases.
                "p" | "pl" | "patch" => return Some(version),
                _ => return None,
            };
            let n = rest.trim_start_matches('.').parse().unwrap_or(0);
            version.pre = vec![Ident::Num(rank), Ident::Num(n)];
        }
        Some(version)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparator {
    pub op: Op,
    pub version: Version,
}

impl Comparator {
    fn new(op: Op, version: Version) -> Self {
        Self { op, version }
    }

    fn test(&self, v: &Version) -> bool {
        let ord = v.cmp(&self.version);
        match self.op {
            Op::Lt => ord == Ordering::Less,
            Op::Le => ord != Ordering::Greater,
            Op::Gt => ord == Ordering::Greater,
            Op::Ge => ord != Ordering::Less,
            Op::Eq => ord == Ordering::Equal,
            Op::Ne => ord != Ordering::Equal,
        }
    }
}

/// An OR of AND-ed comparators. An empty AND-set matches every version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Range {
    pub sets: Vec<Vec<Comparator>>,
}

impl Range {
    pub fn any() -> Self {
        Self {
            sets: vec![Vec::new()],
        }
    }

    /// npm rules: a pre-release only matches when an AND-set names a
    /// pre-release of the same `major.minor.patch` (so `^1.2.0` never
    /// pulls in `2.0.0-beta`).
    pub fn matches(&self, v: &Version) -> bool {
        self.sets.iter().any(|set| {
            set.iter().all(|c| c.test(v))
                && (!v.is_prerelease()
                    || set
                        .iter()
                        .any(|c| c.version.is_prerelease() && c.version.nums == v.nums))
        })
    }

    /// The newest of `versions` that matches.
    pub fn max_matching<'a>(
        &self,
        versions: impl IntoIterator<Item = &'a Version>,
    ) -> Option<&'a Version> {
        versions.into_iter().filter(|v| self.matches(v)).max()
    }

    /// The smallest major any matching version could have, used to tell a
    /// within-range update from a jump to a new major.
    pub fn lowest_major(&self) -> Option<u64> {
        self.sets
            .iter()
            .map(|set| {
                set.iter()
                    .filter(|c| matches!(c.op, Op::Ge | Op::Gt | Op::Eq))
                    .map(|c| c.version.major())
                    .max()
                    .unwrap_or(0)
            })
            .min()
    }
}

/// Parses an npm range (`^1.2.0`, `>=18 <21`, `1.x || 2.x`, `1.2 - 1.4`).
pub fn parse_npm(raw: &str) -> Result<Range, String> {
    let raw = raw.trim();
    if raw.is_empty() || raw == "*" || raw.eq_ignore_ascii_case("x") || raw == "latest" {
        return Ok(Range::any());
    }
    let mut sets = Vec::new();
    for part in raw.split("||") {
        sets.push(parse_set(part.trim(), Flavor::Npm).map_err(|e| format!("'{raw}': {e}"))?);
    }
    Ok(Range { sets })
}

/// Parses a Composer constraint (`^8.2`, `>=7.4 <8.4`, `~2.1 || ^3.0`,
/// `8.1.*`), ignoring stability flags such as `@dev`.
pub fn parse_composer(raw: &str) -> Result<Range, String> {
    let raw = raw.trim();
    if raw.is_empty() || raw == "*" {
        return Ok(Range::any());
    }
    let normalized = raw.replace("||", "|");
    let mut sets = Vec::new();
    for part in normalized.split('|') {
        // Commas and spaces both mean AND.
        let part = part.replace(',', " ");
        sets.push(parse_set(part.trim(), Flavor::Composer).map_err(|e| format!("'{raw}': {e}"))?);
    }
    Ok(Range { sets })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Flavor {
    Npm,
    Composer,
}

fn parse_set(raw: &str, flavor: Flavor) -> Result<Vec<Comparator>, String> {
    if raw.is_empty() || raw == "*" {
        return Ok(Vec::new());
    }
    // Hyphen ranges need spaces around the dash: `1.2 - 1.4`.
    if let Some((low, high)) = raw.split_once(" - ") {
        let low = Partial::parse(low.trim()).ok_or("bad lower bound")?;
        let high = Partial::parse(high.trim()).ok_or("bad upper bound")?;
        let mut out = vec![Comparator::new(Op::Ge, low.floor())];
        out.push(if high.parts < 3 || high.has_wildcard() {
            Comparator::new(Op::Lt, high.bump_last_given())
        } else {
            Comparator::new(Op::Le, high.floor())
        });
        return Ok(out);
    }

    let tokens = tokenize(raw);
    let mut out = Vec::new();
    for token in tokens {
        out.extend(expand(&token, flavor)?);
    }
    Ok(out)
}

/// Splits on whitespace, gluing operators to the version after them
/// (`>= 1.2` → `>=1.2`).
fn tokenize(raw: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut pending = String::new();
    for word in raw.split_whitespace() {
        if word.chars().all(|c| "<>=!^~".contains(c)) {
            pending.push_str(word);
            continue;
        }
        out.push(format!("{pending}{word}"));
        pending.clear();
    }
    // A dangling operator is an error, not "any version".
    if !pending.is_empty() {
        out.push(pending);
    }
    out
}

fn expand(token: &str, flavor: Flavor) -> Result<Vec<Comparator>, String> {
    let token = match flavor {
        // Stability flags (`^1.0@dev`, `@stable`) only affect resolution.
        Flavor::Composer => token.split('@').next().unwrap_or_default(),
        Flavor::Npm => token,
    };
    let (op, rest) = split_op(token);
    let rest = rest.trim();
    if rest.is_empty() {
        return Err(format!("missing version after '{op}'"));
    }
    let partial = match flavor {
        Flavor::Npm => Partial::parse(rest),
        Flavor::Composer => Partial::parse_composer(rest),
    }
    .ok_or_else(|| format!("'{rest}' is not a version"))?;

    let floor = partial.floor();
    // npm reads `1.2` as `1.2.x`; Composer reads it as exactly 1.2.0.
    let exact = match flavor {
        Flavor::Npm => partial.parts >= 3,
        Flavor::Composer => !partial.wild,
    };
    let all = Ok(Vec::new());
    Ok(match op {
        "^" => {
            if partial.parts == 0 {
                return all;
            }
            let upper = partial.caret_upper();
            vec![
                Comparator::new(Op::Ge, floor),
                Comparator::new(Op::Lt, upper),
            ]
        }
        "~" | "~>" | "~=" => {
            if partial.parts == 0 {
                return all;
            }
            // npm: `~1.2` and `~1.2.3` stay in 1.2. Composer: the last given
            // part may move (`~1.2` < 2.0, `~1.2.3` < 1.3).
            let upper = match flavor {
                Flavor::Npm if partial.parts == 1 => partial.bump(0),
                Flavor::Npm => partial.bump(1),
                Flavor::Composer if partial.parts <= 2 => partial.bump(0),
                Flavor::Composer => partial.bump(partial.parts - 2),
            };
            vec![
                Comparator::new(Op::Ge, floor),
                Comparator::new(Op::Lt, upper),
            ]
        }
        "" | "=" | "==" | "v" => {
            if partial.parts == 0 {
                return all;
            }
            if exact {
                vec![Comparator::new(Op::Eq, floor)]
            } else {
                vec![
                    Comparator::new(Op::Ge, floor),
                    Comparator::new(Op::Lt, partial.bump_last_given()),
                ]
            }
        }
        ">=" => vec![Comparator::new(Op::Ge, floor)],
        "<" => vec![Comparator::new(Op::Lt, floor_with_pre(floor))],
        ">" if exact => vec![Comparator::new(Op::Gt, floor)],
        // `>1.2` means "past every 1.2.x".
        ">" => vec![Comparator::new(Op::Ge, partial.bump_last_given_release())],
        "<=" if exact => vec![Comparator::new(Op::Le, floor)],
        "<=" => vec![Comparator::new(Op::Lt, partial.bump_last_given())],
        "!=" => vec![Comparator::new(Op::Ne, floor)],
        other => return Err(format!("unknown operator '{other}'")),
    })
}

/// `<2.0.0` should also exclude `2.0.0-beta`.
fn floor_with_pre(mut v: Version) -> Version {
    if v.pre.is_empty() {
        v.pre = vec![Ident::Num(0)];
    }
    v
}

fn split_op(token: &str) -> (&str, &str) {
    for op in [">=", "<=", "==", "!=", "~>", "~=", ">", "<", "=", "^", "~"] {
        if let Some(rest) = token.strip_prefix(op) {
            return (op, rest);
        }
    }
    ("", token)
}

/// A possibly partial version: `1`, `1.2`, `1.2.x`, `1.2.3-beta`.
#[derive(Debug, Clone)]
struct Partial {
    /// `None` is a wildcard (`x`, `*`).
    nums: [Option<u64>; 4],
    /// How many numeric parts were written (a wildcard ends the count).
    parts: usize,
    /// Whether a wildcard (`x`, `*`) was written.
    wild: bool,
    pre: Vec<Ident>,
}

impl Partial {
    fn parse(raw: &str) -> Option<Self> {
        let raw = raw.trim().trim_start_matches(['v', 'V', '=']).trim();
        if raw.is_empty() {
            return None;
        }
        let raw = raw.split('+').next().unwrap_or_default();
        let (core, pre) = match raw.split_once('-') {
            Some((core, pre)) => (core, Some(pre)),
            None => (raw, None),
        };
        let mut nums = [Some(0); 4];
        let mut parts = 0;
        for (i, piece) in core.split('.').enumerate() {
            if i >= 4 {
                return None;
            }
            nums[i] = match piece {
                "x" | "X" | "*" => None,
                n if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => {
                    Some(n.parse().ok()?)
                }
                _ => return None,
            };
            parts = i + 1;
        }
        // Everything after the first wildcard is a wildcard too.
        let wild = nums[..parts].iter().any(Option::is_none);
        if let Some(first) = nums[..parts].iter().position(Option::is_none) {
            for n in &mut nums[first..] {
                *n = None;
            }
            parts = first;
        }
        let pre = match pre {
            Some(pre) => pre
                .split('.')
                .map(|p| match p.parse() {
                    Ok(n) if !p.is_empty() => Some(Ident::Num(n)),
                    _ if !p.is_empty()
                        && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') =>
                    {
                        Some(Ident::Text(p.to_string()))
                    }
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()?,
            None => Vec::new(),
        };
        Some(Self {
            nums,
            parts,
            wild,
            pre,
        })
    }

    fn parse_composer(raw: &str) -> Option<Self> {
        let raw = raw.trim().trim_start_matches(['v', 'V']);
        if raw.contains(['*', 'x', 'X']) {
            return Self::parse(raw);
        }
        let version = Version::parse_composer(raw)?;
        let core = raw.split(['-', '+']).next().unwrap_or_default();
        let parts = core.split('.').count().min(4);
        let mut nums = [Some(0); 4];
        for (i, n) in version.nums.iter().enumerate() {
            nums[i] = Some(*n);
        }
        Some(Self {
            nums,
            parts,
            wild: false,
            pre: version.pre,
        })
    }

    fn has_wildcard(&self) -> bool {
        self.wild || self.parts < 3
    }

    fn floor(&self) -> Version {
        Version {
            nums: self.nums.map(|n| n.unwrap_or(0)),
            pre: self.pre.clone(),
        }
    }

    /// Increments part `i` and zeroes the rest, as an exclusive upper bound.
    fn bump(&self, i: usize) -> Version {
        let mut nums = self.nums.map(|n| n.unwrap_or(0));
        nums[i] += 1;
        for n in &mut nums[i + 1..] {
            *n = 0;
        }
        Version {
            nums,
            pre: vec![Ident::Num(0)],
        }
    }

    fn bump_last_given(&self) -> Version {
        self.bump(self.parts.saturating_sub(1).min(3))
    }

    fn bump_last_given_release(&self) -> Version {
        let mut v = self.bump_last_given();
        v.pre.clear();
        v
    }

    /// `^`: the first non-zero part given may not change.
    fn caret_upper(&self) -> Version {
        let nums = self.nums.map(|n| n.unwrap_or(0));
        let given = self.parts.min(3);
        let first_nonzero = (0..given).find(|&i| nums[i] != 0);
        match first_nonzero {
            Some(i) => self.bump(i),
            // `^0.0.3` → <0.0.4, `^0.0` → <0.1, `^0` → <1.
            None => self.bump(given.saturating_sub(1)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(raw: &str) -> Version {
        Version::parse(raw).unwrap()
    }

    fn npm(range: &str, version: &str) -> bool {
        parse_npm(range).unwrap().matches(&v(version))
    }

    fn composer(range: &str, version: &str) -> bool {
        parse_composer(range)
            .unwrap()
            .matches(&Version::parse_composer(version).unwrap())
    }

    #[test]
    fn orders_versions() {
        assert!(v("1.10.0") > v("1.9.9"));
        assert!(v("1.0.0") > v("1.0.0-rc.1"));
        assert!(v("1.0.0-alpha.1") > v("1.0.0-alpha"));
        assert!(v("1.0.0-alpha.beta") > v("1.0.0-alpha.1"));
        assert!(v("v2.0.0") == v("2.0.0+build.5"));
        assert_eq!(v("1.2.3-beta.2").to_string(), "1.2.3-beta.2");
    }

    #[test]
    fn npm_caret_and_tilde() {
        assert!(npm("^1.2.3", "1.9.0"));
        assert!(!npm("^1.2.3", "2.0.0"));
        assert!(!npm("^1.2.3", "1.2.2"));
        assert!(npm("^0.2.3", "0.2.9"));
        assert!(!npm("^0.2.3", "0.3.0"));
        assert!(!npm("^0.0.3", "0.0.4"));
        assert!(npm("^1.x", "1.5.0"));
        assert!(npm("~1.2.3", "1.2.9"));
        assert!(!npm("~1.2.3", "1.3.0"));
        assert!(npm("~1", "1.9.0"));
        assert!(!npm("~1.2", "1.3.0"));
    }

    #[test]
    fn npm_comparators_and_unions() {
        assert!(npm(">=18", "22.11.0"));
        assert!(!npm(">=18 <21", "22.0.0"));
        assert!(npm(">= 18.0.0 < 21", "20.3.1"));
        assert!(npm("^18 || ^20 || >=22", "22.1.0"));
        assert!(!npm("^18 || ^20", "19.0.0"));
        assert!(npm("1.2 - 1.4", "1.4.9"));
        assert!(!npm("1.2 - 1.4", "1.5.0"));
        assert!(npm("1.2.3 - 2.3.4", "2.3.4"));
        assert!(npm("1.x", "1.0.5"));
        assert!(!npm(">1.2", "1.2.9"));
        assert!(npm(">1.2", "1.3.0"));
        assert!(npm("<=1.2", "1.2.7"));
        assert!(npm("*", "0.0.1"));
        assert!(npm("", "3.0.0"));
    }

    #[test]
    fn npm_prereleases_need_opt_in() {
        assert!(!npm("^1.2.0", "1.3.0-beta.1"));
        assert!(!npm("^1.2.0", "2.0.0-beta"));
        assert!(npm("^1.2.0-beta.1", "1.2.0-beta.3"));
        assert!(!npm("^1.2.0-beta.1", "1.3.0-beta.1"));
        assert!(!npm("<2.0.0", "2.0.0-rc.1"));
    }

    #[test]
    fn rejects_non_ranges() {
        for raw in [
            "workspace:*",
            "file:../x",
            "github:user/repo",
            "next",
            "^",
            ">=",
            "1.2.3.4.5",
        ] {
            assert!(parse_npm(raw).is_err(), "{raw}");
        }
        assert!(parse_composer("dev-main").is_err());
    }

    #[test]
    fn composer_constraints() {
        assert!(composer("^8.2", "8.3.14"));
        assert!(!composer("^8.2", "9.0.0"));
        assert!(!composer("^8.2", "8.1.30"));
        // Composer's tilde lets the second-to-last part move.
        assert!(composer("~1.2", "1.9.0"));
        assert!(!composer("~1.2", "2.0.0"));
        assert!(composer("~1.2.3", "1.2.9"));
        assert!(!composer("~1.2.3", "1.3.0"));
        assert!(composer(">=7.4 <8.4", "8.3.0"));
        assert!(composer(">=7.4,<8.4", "7.4.33"));
        assert!(!composer(">=7.4,<8.4", "8.4.0"));
        assert!(composer("^7.4 || ^8.0", "8.2.1"));
        assert!(composer("^7.4|^8.0", "7.4.0"));
        assert!(composer("8.1.*", "8.1.27"));
        assert!(!composer("8.1.*", "8.2.0"));
        assert!(composer("^2.0@dev", "2.3.0"));
        assert!(composer("1.0.0.1", "1.0.0.1"));
        // Without a wildcard a Composer version is exact.
        assert!(composer("8.2", "8.2.0"));
        assert!(!composer("8.2", "8.2.5"));
        assert!(composer(">8.2", "8.2.1"));
    }

    #[test]
    fn composer_stability_order() {
        let p = |s| Version::parse_composer(s).unwrap();
        assert!(p("1.0.0-dev") < p("1.0.0-alpha1"));
        assert!(p("1.0.0-alpha1") < p("1.0.0-beta"));
        assert!(p("1.0.0-beta2") < p("1.0.0-RC1"));
        assert!(p("1.0.0-RC1") < p("1.0.0"));
        assert!(!composer("^1.0", "1.1.0-beta1"));
    }

    #[test]
    fn finds_lowest_major_and_best_match() {
        assert_eq!(parse_npm("^18 || ^20").unwrap().lowest_major(), Some(18));
        assert_eq!(parse_npm("*").unwrap().lowest_major(), Some(0));
        let all = [v("1.0.0"), v("1.4.2"), v("2.0.0"), v("1.5.0-beta")];
        let range = parse_npm("^1").unwrap();
        assert_eq!(range.max_matching(&all), Some(&all[1]));
    }
}
