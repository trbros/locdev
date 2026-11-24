//! Just enough of the DNS wire format (RFC 1035) to be authoritative for a
//! single local TLD: every name under it is an `A` record for 127.0.0.1.

const HEADER_LEN: usize = 12;
const MAX_NAME_LEN: usize = 255;
const TTL: u32 = 60;

const TYPE_A: u16 = 1;
const TYPE_SOA: u16 = 6;
const TYPE_OPT: u16 = 41;
const TYPE_ANY: u16 = 255;
const CLASS_IN: u16 = 1;

const FLAG_QR: u16 = 0x8000;
const FLAG_AA: u16 = 0x0400;
const FLAG_RD: u16 = 0x0100;
const FLAG_RA: u16 = 0x0080;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Rcode {
    NoError = 0,
    FormErr = 1,
    NotImp = 4,
    Refused = 5,
}

/// A parsed question plus the raw bytes needed to echo it back.
#[derive(Debug)]
struct Question<'a> {
    /// Lower-cased, dot-separated, without the trailing dot.
    name: String,
    qtype: u16,
    qclass: u16,
    raw: &'a [u8],
    /// Whether the query carried an EDNS(0) OPT record.
    edns: bool,
}

/// Builds the response for `query`. Returns the response bytes and the name
/// that was asked about, or `None` for packets too broken to answer.
pub fn respond(query: &[u8], tld: &str) -> Option<(Vec<u8>, Option<String>)> {
    if query.len() < HEADER_LEN {
        return None;
    }
    let id = u16::from_be_bytes([query[0], query[1]]);
    let flags = u16::from_be_bytes([query[2], query[3]]);
    if flags & FLAG_QR != 0 {
        return None; // A response, not a query: never answer those.
    }
    let opcode = (flags >> 11) & 0x0f;
    let qdcount = u16::from_be_bytes([query[4], query[5]]);
    let arcount = u16::from_be_bytes([query[10], query[11]]);

    let mut out = Response::new(id, flags);

    if opcode != 0 {
        return Some((out.finish(Rcode::NotImp, None), None));
    }
    if qdcount != 1 {
        return Some((out.finish(Rcode::FormErr, None), None));
    }
    let Some(question) = parse_question(query, arcount) else {
        return Some((out.finish(Rcode::FormErr, None), None));
    };

    out.edns = question.edns;
    let name = question.name.clone();
    let in_zone =
        question.qclass == CLASS_IN && (name == tld || name.ends_with(&format!(".{tld}")));

    if !in_zone {
        return Some((out.finish(Rcode::Refused, Some(&question)), Some(name)));
    }

    if matches!(question.qtype, TYPE_A | TYPE_ANY) {
        out.answer_a();
    } else if question.qtype == TYPE_SOA && name == tld {
        out.answer_soa(tld);
    } else {
        // NODATA: the name exists, just not with this type (e.g. AAAA). The
        // SOA lets resolvers cache the negative answer.
        out.authority_soa(tld);
    }

    Some((out.finish(Rcode::NoError, Some(&question)), Some(name)))
}

fn parse_question(query: &[u8], arcount: u16) -> Option<Question<'_>> {
    let mut pos = HEADER_LEN;
    let mut labels = Vec::new();
    let mut name_len = 0;

    loop {
        let len = *query.get(pos)? as usize;
        pos += 1;
        if len == 0 {
            break;
        }
        // Compression pointers are not valid in a question we have to echo.
        if len & 0xc0 != 0 {
            return None;
        }
        let label = query.get(pos..pos + len)?;
        name_len += len + 1;
        if name_len > MAX_NAME_LEN {
            return None;
        }
        labels.push(String::from_utf8_lossy(label).to_ascii_lowercase());
        pos += len;
    }

    let fixed = query.get(pos..pos + 4)?;
    let qtype = u16::from_be_bytes([fixed[0], fixed[1]]);
    let qclass = u16::from_be_bytes([fixed[2], fixed[3]]);
    let raw = &query[HEADER_LEN..pos + 4];

    // An OPT pseudo-record is a root name (0) followed by type 41.
    let rest = &query[pos + 4..];
    let edns = arcount > 0
        && rest.len() >= 3
        && rest[0] == 0
        && u16::from_be_bytes([rest[1], rest[2]]) == TYPE_OPT;

    Some(Question {
        name: labels.join("."),
        qtype,
        qclass,
        raw,
        edns,
    })
}

struct Response {
    id: u16,
    flags: u16,
    answers: Vec<u8>,
    ancount: u16,
    authority: Vec<u8>,
    nscount: u16,
    edns: bool,
}

impl Response {
    fn new(id: u16, query_flags: u16) -> Self {
        Self {
            id,
            // Keep the opcode and RD bit from the query.
            flags: FLAG_QR | FLAG_AA | FLAG_RA | (query_flags & (0x7800 | FLAG_RD)),
            answers: Vec::new(),
            ancount: 0,
            authority: Vec::new(),
            nscount: 0,
            edns: false,
        }
    }

    fn answer_a(&mut self) {
        // Name: pointer to the question name at offset 12.
        self.answers.extend_from_slice(&[0xc0, 0x0c]);
        push_rr_header(&mut self.answers, TYPE_A, TTL, 4);
        self.answers.extend_from_slice(&[127, 0, 0, 1]);
        self.ancount += 1;
    }

    fn answer_soa(&mut self, tld: &str) {
        self.answers.extend_from_slice(&[0xc0, 0x0c]);
        push_soa_rdata(&mut self.answers, tld);
        self.ancount += 1;
    }

    fn authority_soa(&mut self, tld: &str) {
        push_name(&mut self.authority, tld);
        push_soa_rdata(&mut self.authority, tld);
        self.nscount += 1;
    }

    fn finish(self, rcode: Rcode, question: Option<&Question>) -> Vec<u8> {
        let mut out = Vec::with_capacity(512);
        out.extend_from_slice(&self.id.to_be_bytes());
        out.extend_from_slice(&(self.flags | rcode as u16).to_be_bytes());
        out.extend_from_slice(&u16::from(question.is_some()).to_be_bytes());
        out.extend_from_slice(&self.ancount.to_be_bytes());
        out.extend_from_slice(&self.nscount.to_be_bytes());
        out.extend_from_slice(&u16::from(self.edns).to_be_bytes());

        if let Some(question) = question {
            out.extend_from_slice(question.raw);
        }
        out.extend_from_slice(&self.answers);
        out.extend_from_slice(&self.authority);

        if self.edns {
            // Root name, type OPT, 1232-byte UDP payload, no flags, no options.
            out.push(0);
            out.extend_from_slice(&TYPE_OPT.to_be_bytes());
            out.extend_from_slice(&1232u16.to_be_bytes());
            out.extend_from_slice(&0u32.to_be_bytes());
            out.extend_from_slice(&0u16.to_be_bytes());
        }
        out
    }
}

fn push_rr_header(out: &mut Vec<u8>, rtype: u16, ttl: u32, rdlen: u16) {
    out.extend_from_slice(&rtype.to_be_bytes());
    out.extend_from_slice(&CLASS_IN.to_be_bytes());
    out.extend_from_slice(&ttl.to_be_bytes());
    out.extend_from_slice(&rdlen.to_be_bytes());
}

fn push_name(out: &mut Vec<u8>, name: &str) {
    for label in name.split('.').filter(|l| !l.is_empty()) {
        out.push(label.len() as u8);
        out.extend_from_slice(label.as_bytes());
    }
    out.push(0);
}

fn push_soa_rdata(out: &mut Vec<u8>, tld: &str) {
    let mut rdata = Vec::new();
    push_name(&mut rdata, &format!("ns.locdev.{tld}"));
    push_name(&mut rdata, &format!("hostmaster.locdev.{tld}"));
    // serial, refresh, retry, expire, minimum (negative-caching TTL)
    for value in [1u32, 3600, 600, 86400, TTL] {
        rdata.extend_from_slice(&value.to_be_bytes());
    }
    push_rr_header(out, TYPE_SOA, TTL, rdata.len() as u16);
    out.extend_from_slice(&rdata);
}

/// Builds a query packet. Used by tests and by the daemon's self-check.
pub fn query(id: u16, name: &str, qtype: u16, edns: bool) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&id.to_be_bytes());
    out.extend_from_slice(&FLAG_RD.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&u16::from(edns).to_be_bytes());
    push_name(&mut out, name);
    out.extend_from_slice(&qtype.to_be_bytes());
    out.extend_from_slice(&CLASS_IN.to_be_bytes());
    if edns {
        out.push(0);
        out.extend_from_slice(&TYPE_OPT.to_be_bytes());
        out.extend_from_slice(&1232u16.to_be_bytes());
        out.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const AAAA: u16 = 28;

    fn header(resp: &[u8]) -> (u16, u16, [u16; 4]) {
        let word = |i: usize| u16::from_be_bytes([resp[i], resp[i + 1]]);
        (word(0), word(2), [word(4), word(6), word(8), word(10)])
    }

    #[test]
    fn answers_a_for_names_under_the_tld() {
        for name in ["blog.test", "api.Blog.TEST", "test"] {
            let (resp, asked) = respond(&query(0xbeef, name, TYPE_A, false), "test").unwrap();
            let (id, flags, counts) = header(&resp);

            assert_eq!(id, 0xbeef);
            assert_eq!(flags & 0x000f, Rcode::NoError as u16);
            assert_ne!(flags & FLAG_QR, 0);
            assert_ne!(flags & FLAG_AA, 0);
            assert_eq!(counts, [1, 1, 0, 0], "{name}");
            assert_eq!(&resp[resp.len() - 4..], &[127, 0, 0, 1]);
            assert_eq!(asked.unwrap(), name.to_ascii_lowercase());
        }
    }

    #[test]
    fn aaaa_is_nodata_with_soa() {
        let (resp, _) = respond(&query(1, "blog.test", AAAA, false), "test").unwrap();
        let (_, flags, counts) = header(&resp);

        assert_eq!(flags & 0x000f, Rcode::NoError as u16);
        assert_eq!(counts, [1, 0, 1, 0]);
    }

    #[test]
    fn refuses_other_zones() {
        let (resp, _) = respond(&query(2, "example.com", TYPE_A, false), "test").unwrap();
        let (_, flags, counts) = header(&resp);

        assert_eq!(flags & 0x000f, Rcode::Refused as u16);
        assert_eq!(counts, [1, 0, 0, 0]);

        // "mytest" is not under "test".
        let (resp, _) = respond(&query(3, "mytest", TYPE_A, false), "test").unwrap();
        assert_eq!(header(&resp).1 & 0x000f, Rcode::Refused as u16);
    }

    #[test]
    fn echoes_edns() {
        let (resp, _) = respond(&query(4, "blog.test", TYPE_A, true), "test").unwrap();
        let (_, _, counts) = header(&resp);

        assert_eq!(counts, [1, 1, 0, 1]);
        assert_eq!(&resp[resp.len() - 11..resp.len() - 8], &[0, 0, 41]);
    }

    #[test]
    fn rejects_garbage() {
        assert!(respond(&[0; 5], "test").is_none());

        // A response packet must never be answered (avoids reflection loops).
        let mut packet = query(5, "blog.test", TYPE_A, false);
        packet[2] |= 0x80;
        assert!(respond(&packet, "test").is_none());

        // Truncated question.
        let packet = query(6, "blog.test", TYPE_A, false);
        let (resp, _) = respond(&packet[..packet.len() - 3], "test").unwrap();
        assert_eq!(header(&resp).1 & 0x000f, Rcode::FormErr as u16);
    }
}
