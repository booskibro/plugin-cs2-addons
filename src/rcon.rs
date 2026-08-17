//! CS2-tolerant Source RCON engine.
//!
//! The panel's built-in Source client reads exactly one response packet and
//! rejects anything over 4096 bytes — assumptions CS2 breaks routinely: it
//! splits long replies across packets, occasionally exceeds the classic size
//! cap, pushes unsolicited console output onto authenticated connections, and
//! answers auth either with a bare AUTH_RESPONSE or the classic empty
//! RESPONSE_VALUE + AUTH_RESPONSE pair. This engine tolerates all of that.
//!
//! Transport-agnostic: the wasm build drives it over the gameap-net host
//! library (see `wasm_ext`), tests over scripted byte streams.

pub const TYPE_RESPONSE_VALUE: i32 = 0;
pub const TYPE_EXEC_COMMAND: i32 = 2;
pub const TYPE_AUTH_RESPONSE: i32 = 2;
pub const TYPE_AUTH: i32 = 3;

/// Well past anything CS2 sends, small enough to bound the wasm heap.
const MAX_PACKET_BODY: usize = 8 * 1024 * 1024;
const MAX_TOTAL_OUTPUT: usize = 8 * 1024 * 1024;
const MAX_PACKETS_PER_EXCHANGE: usize = 256;

const RECV_CHUNK: u32 = 60 * 1024;
/// First-response patience; the host clamps to its own ceiling anyway.
const FIRST_RESPONSE_TIMEOUT_MS: u32 = 5_000;
/// Idle gap that ends a response once something has arrived (the fallback
/// stop when a server does not echo the end marker).
const IDLE_TIMEOUT_MS: u32 = 600;

pub struct RecvChunk {
    pub data: Vec<u8>,
    pub timeout: bool,
}

/// One TCP-ish byte stream. `recv` returns an empty timeout chunk when the
/// deadline passes without data.
pub trait Wire {
    fn send(&mut self, data: &[u8]) -> Result<(), String>;
    fn recv(&mut self, max_bytes: u32, timeout_ms: u32) -> Result<RecvChunk, String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet {
    pub id: i32,
    pub ptype: i32,
    pub body: Vec<u8>,
}

pub fn encode_packet(id: i32, ptype: i32, body: &[u8]) -> Vec<u8> {
    let size = (4 + 4 + body.len() + 2) as u32;
    let mut out = Vec::with_capacity(size as usize + 4);
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&id.to_le_bytes());
    out.extend_from_slice(&ptype.to_le_bytes());
    out.extend_from_slice(body);
    out.push(0);
    out.push(0);
    out
}

/// Parses one packet off the front of `buf`; returns the bytes consumed.
/// `Ok(None)` = incomplete, wait for more bytes.
fn try_parse_packet(buf: &[u8]) -> Result<Option<(Packet, usize)>, String> {
    if buf.len() < 4 {
        return Ok(None);
    }
    let size = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    if !(10..=MAX_PACKET_BODY + 10).contains(&size) {
        return Err(format!("invalid rcon packet size {size}"));
    }
    if buf.len() < 4 + size {
        return Ok(None);
    }
    let id = i32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
    let ptype = i32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);
    // Body runs to the packet end minus the two null terminators; a server
    // that omits them is tolerated (body simply runs to the end).
    let body_end = 4 + size - if size >= 12 { 2 } else { 0 };
    let body = buf[12..body_end.max(12)].to_vec();
    Ok(Some((Packet { id, ptype, body }, 4 + size)))
}

/// Byte-stream reassembly buffer, carried across reads on one connection.
#[derive(Default)]
pub struct Session {
    buf: Vec<u8>,
}

impl Session {
    /// Next packet: drains the buffer first, then reads with `timeout_ms`.
    /// `Ok(None)` = the wire went idle for the full timeout.
    fn next_packet<W: Wire>(
        &mut self,
        wire: &mut W,
        timeout_ms: u32,
    ) -> Result<Option<Packet>, String> {
        loop {
            if let Some((packet, consumed)) = try_parse_packet(&self.buf)? {
                self.buf.drain(..consumed);
                return Ok(Some(packet));
            }
            let chunk = wire.recv(RECV_CHUNK, timeout_ms)?;
            if chunk.timeout && chunk.data.is_empty() {
                return Ok(None);
            }
            self.buf.extend_from_slice(&chunk.data);
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum AuthOutcome {
    Ok,
    BadPassword,
}

/// Authenticates. Skips any RESPONSE_VALUE packets before the AUTH_RESPONSE
/// (classic srcds sends an empty one; CS2 may not).
pub fn authenticate<W: Wire>(
    wire: &mut W,
    session: &mut Session,
    password: &str,
    request_id: i32,
) -> Result<AuthOutcome, String> {
    wire.send(&encode_packet(request_id, TYPE_AUTH, password.as_bytes()))?;
    for _ in 0..MAX_PACKETS_PER_EXCHANGE {
        match session.next_packet(wire, FIRST_RESPONSE_TIMEOUT_MS)? {
            Some(packet) if packet.ptype == TYPE_AUTH_RESPONSE => {
                return if packet.id == request_id {
                    Ok(AuthOutcome::Ok)
                } else {
                    // -1 per spec; any foreign id on the auth answer means no.
                    Ok(AuthOutcome::BadPassword)
                };
            }
            Some(_) => continue, // pre-auth RESPONSE_VALUE or console noise
            None => return Err("server did not answer the auth request".into()),
        }
    }
    Err("no auth response among the first packets".into())
}

/// What one command exchange produced, with enough detail to diagnose
/// nonstandard servers from the panel log.
#[derive(Debug, Default)]
pub struct ExecOutcome {
    pub output: String,
    /// The output came from RESPONSE_VALUE packets carrying a FOREIGN request
    /// id — some CS2 builds do not echo the client's id on command output.
    pub fallback_used: bool,
    pub own_packets: u32,
    pub foreign_packets: u32,
}

/// Runs one command and returns its full (possibly multi-packet) output.
///
/// NO end-marker request is sent. The classic srcds trick (an empty
/// RESPONSE_VALUE pipelined behind the command, echoed back in order) makes
/// some CS2 builds DISCARD the pending command's output — the field signature
/// is auth passing while every command, `status` included, returns empty,
/// with plain single-request external tools working fine against the same
/// server. The response is delimited by an idle gap instead, which the
/// multi-packet reads already handle.
///
/// RESPONSE_VALUE packets with foreign ids are buffered separately: normally
/// they are CS2 console streaming and are discarded, but when a server
/// returns NOTHING under our id they are the command output under a
/// nonstandard id, and they win over an empty answer.
pub fn execute<W: Wire>(
    wire: &mut W,
    session: &mut Session,
    command: &str,
    request_id: i32,
) -> Result<ExecOutcome, String> {
    wire.send(&encode_packet(request_id, TYPE_EXEC_COMMAND, command.as_bytes()))?;

    let mut output: Vec<u8> = Vec::new();
    let mut foreign: Vec<u8> = Vec::new();
    let mut own_packets = 0u32;
    let mut foreign_packets = 0u32;
    for _ in 0..MAX_PACKETS_PER_EXCHANGE {
        let received_any = own_packets > 0 || foreign_packets > 0;
        let timeout_ms = if received_any {
            IDLE_TIMEOUT_MS
        } else {
            FIRST_RESPONSE_TIMEOUT_MS
        };
        match session.next_packet(wire, timeout_ms)? {
            Some(packet) if packet.id == request_id && packet.ptype == TYPE_RESPONSE_VALUE => {
                own_packets += 1;
                if output.len() + packet.body.len() > MAX_TOTAL_OUTPUT {
                    return Err("response exceeds the output cap".into());
                }
                output.extend_from_slice(&packet.body);
            }
            Some(packet) if packet.ptype == TYPE_RESPONSE_VALUE => {
                foreign_packets += 1;
                if foreign.len() + packet.body.len() <= MAX_TOTAL_OUTPUT {
                    foreign.extend_from_slice(&packet.body);
                }
            }
            Some(_) => continue, // auth responses, stale packets
            None if received_any => break, // idle gap = response complete
            // Total silence is a legitimate answer to a silent command
            // (css_plugins load, exec …) — CS2 sends nothing at all for some
            // of them. Empty output, not an error.
            None => break,
        }
    }

    let fallback_used = output.is_empty() && !foreign.is_empty();
    let bytes = if fallback_used { &foreign } else { &output };
    Ok(ExecOutcome {
        output: String::from_utf8_lossy(bytes).into_owned(),
        fallback_used,
        own_packets,
        foreign_packets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wire fed from a script of receive chunks; sends are recorded.
    #[derive(Default)]
    struct ScriptedWire {
        sent: Vec<Vec<u8>>,
        chunks: std::collections::VecDeque<Option<Vec<u8>>>, // None = timeout
    }

    impl ScriptedWire {
        fn push_packet(&mut self, id: i32, ptype: i32, body: &[u8]) {
            self.chunks.push_back(Some(encode_packet(id, ptype, body)));
        }
        fn push_timeout(&mut self) {
            self.chunks.push_back(None);
        }
        fn push_raw(&mut self, bytes: Vec<u8>) {
            self.chunks.push_back(Some(bytes));
        }
    }

    impl Wire for ScriptedWire {
        fn send(&mut self, data: &[u8]) -> Result<(), String> {
            self.sent.push(data.to_vec());
            Ok(())
        }
        fn recv(&mut self, _max: u32, _timeout: u32) -> Result<RecvChunk, String> {
            match self.chunks.pop_front() {
                Some(Some(data)) => Ok(RecvChunk {
                    data,
                    timeout: false,
                }),
                Some(None) | None => Ok(RecvChunk {
                    data: Vec::new(),
                    timeout: true,
                }),
            }
        }
    }

    #[test]
    fn auth_bare_response() {
        let mut wire = ScriptedWire::default();
        wire.push_packet(7, TYPE_AUTH_RESPONSE, b"");
        let mut session = Session::default();
        assert_eq!(
            authenticate(&mut wire, &mut session, "pw", 7).expect("auth"),
            AuthOutcome::Ok
        );
    }

    #[test]
    fn auth_classic_pair() {
        let mut wire = ScriptedWire::default();
        wire.push_packet(7, TYPE_RESPONSE_VALUE, b"");
        wire.push_packet(7, TYPE_AUTH_RESPONSE, b"");
        let mut session = Session::default();
        assert_eq!(
            authenticate(&mut wire, &mut session, "pw", 7).expect("auth"),
            AuthOutcome::Ok
        );
    }

    #[test]
    fn auth_rejected() {
        let mut wire = ScriptedWire::default();
        wire.push_packet(7, TYPE_RESPONSE_VALUE, b"");
        wire.push_packet(-1, TYPE_AUTH_RESPONSE, b"");
        let mut session = Session::default();
        assert_eq!(
            authenticate(&mut wire, &mut session, "pw", 7).expect("auth"),
            AuthOutcome::BadPassword
        );
    }

    #[test]
    fn exec_sends_only_the_command() {
        // No pipelined marker request: some CS2 builds discard the pending
        // command's output when a second request arrives behind it.
        let mut wire = ScriptedWire::default();
        wire.push_packet(10, TYPE_RESPONSE_VALUE, b"hello");
        wire.push_timeout();
        let mut session = Session::default();
        let out = execute(&mut wire, &mut session, "echo", 10).expect("exec").output;
        assert_eq!(out, "hello");
        assert_eq!(wire.sent.len(), 1, "exactly one request on the wire");
    }

    #[test]
    fn exec_multi_packet_reassembly() {
        let mut wire = ScriptedWire::default();
        wire.push_packet(10, TYPE_RESPONSE_VALUE, b"part1 ");
        wire.push_packet(10, TYPE_RESPONSE_VALUE, b"part2 ");
        wire.push_packet(10, TYPE_RESPONSE_VALUE, b"part3");
        wire.push_timeout();
        let mut session = Session::default();
        let out = execute(&mut wire, &mut session, "meta list", 10).expect("exec").output;
        assert_eq!(out, "part1 part2 part3");
    }

    #[test]
    fn exec_oversized_packet_ok() {
        // 100KB in one packet — far over the classic 4096 cap.
        let big = vec![b'x'; 100 * 1024];
        let mut wire = ScriptedWire::default();
        wire.push_packet(10, TYPE_RESPONSE_VALUE, &big);
        wire.push_timeout();
        let mut session = Session::default();
        let out = execute(&mut wire, &mut session, "cvarlist", 10).expect("exec").output;
        assert_eq!(out.len(), big.len());
    }

    #[test]
    fn exec_handles_split_and_coalesced_frames() {
        // Two packets delivered as: half of A / rest of A + all of B.
        let a = encode_packet(10, TYPE_RESPONSE_VALUE, b"split ");
        let b = encode_packet(10, TYPE_RESPONSE_VALUE, b"frames");
        let mut wire = ScriptedWire::default();
        wire.push_raw(a[..7].to_vec());
        let mut rest = a[7..].to_vec();
        rest.extend_from_slice(&b);
        wire.push_raw(rest);
        wire.push_timeout();
        let mut session = Session::default();
        let out = execute(&mut wire, &mut session, "x", 10).expect("exec").output;
        assert_eq!(out, "split frames");
    }

    #[test]
    fn exec_foreign_id_output_used_when_nothing_matches() {
        // Some CS2 builds answer command output under a DIFFERENT id than the
        // request. With zero own-id packets, the foreign output must win over
        // an empty answer.
        let mut wire = ScriptedWire::default();
        wire.push_packet(0, TYPE_RESPONSE_VALUE, b"hostname: friendserver");
        wire.push_timeout();
        let mut session = Session::default();
        let outcome = execute(&mut wire, &mut session, "status", 10).expect("exec");
        assert_eq!(outcome.output, "hostname: friendserver");
        assert!(outcome.fallback_used);
        assert_eq!(outcome.foreign_packets, 1);
    }

    #[test]
    fn exec_own_id_output_beats_console_noise() {
        // When own-id packets exist, foreign packets stay classified as noise.
        let mut wire = ScriptedWire::default();
        wire.push_packet(0, TYPE_RESPONSE_VALUE, b"[Server] noise\n");
        wire.push_packet(10, TYPE_RESPONSE_VALUE, b"real answer");
        wire.push_packet(0, TYPE_RESPONSE_VALUE, b"more noise\n");
        wire.push_timeout();
        let mut session = Session::default();
        let outcome = execute(&mut wire, &mut session, "status", 10).expect("exec");
        assert_eq!(outcome.output, "real answer");
        assert!(!outcome.fallback_used);
    }

    #[test]
    fn exec_total_silence_is_empty_not_error() {
        // CS2 sends nothing at all for some silent commands.
        let mut wire = ScriptedWire::default();
        wire.push_timeout();
        let mut session = Session::default();
        let outcome = execute(&mut wire, &mut session, "exec cfg", 10).expect("exec");
        assert_eq!(outcome.output, "");
        assert_eq!(outcome.own_packets, 0);
    }

    #[test]
    fn garbage_size_field_errors() {
        let mut wire = ScriptedWire::default();
        wire.push_raw(vec![0xFF, 0xFF, 0xFF, 0xFF, 1, 2, 3]);
        let mut session = Session::default();
        assert!(execute(&mut wire, &mut session, "x", 10).is_err());
    }
}
