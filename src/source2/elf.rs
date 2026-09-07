//! Just enough ELF64 to read and clear the executable-stack flag.
//!
//! A shared object declares whether it needs an executable stack in the
//! `PT_GNU_STACK` program header. Kernels and glibc used to grant that on
//! `dlopen`; current ones refuse, and the load fails with
//!
//! ```text
//! cannot enable executable stack as shared object requires: Invalid argument
//! ```
//!
//! CounterStrikeSharp's released `counterstrikesharp.so` carries the flag
//! (verified on a real install: `PT_GNU_STACK` flags `RWE`), so reinstalling
//! fetches the same flagged file and changes nothing. The remedy is to clear
//! `PF_X` on that one header — exactly what `execstack -c` does, and what
//! `patchelf --clear-execstack` does — which is a four-byte edit at a computed
//! offset rather than a rewrite of the library.
//!
//! Computing the offset here rather than shelling out matters: `execstack`
//! shipped in `prelink`, which recent distributions dropped, so the documented
//! fix is unavailable on exactly the systems new enough to need it. This module
//! only locates the four bytes; `handlers::execstack` puts them on the node.

pub const PT_GNU_STACK: u32 = 0x6474_e551;
pub const PF_X: u32 = 0x1;

const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
const CLASS_64: u8 = 2;
const DATA_LITTLE_ENDIAN: u8 = 1;
/// Offsets into the ELF64 header.
const E_PHOFF: usize = 0x20;
const E_PHENTSIZE: usize = 0x36;
const E_PHNUM: usize = 0x38;
/// A 64-bit program header is 56 bytes: p_type, p_flags, then eight-byte fields.
const PH_ENTRY_MIN: usize = 56;
const P_FLAGS: usize = 4;

/// Where the `PT_GNU_STACK` flags live, and what they currently say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GnuStack {
    /// Byte offset of the four-byte `p_flags` field within the file.
    pub flags_offset: usize,
    pub flags: u32,
}

impl GnuStack {
    pub fn executable(self) -> bool {
        self.flags & PF_X != 0
    }
}

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn u64_at(bytes: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(bytes.get(at..at + 8)?.try_into().ok()?))
}

/// Locates the `PT_GNU_STACK` header. `Ok(None)` means a valid ELF64 that
/// simply has no such header — nothing to fix, rather than a failure.
///
/// Every field is bounds-checked before use: this parses a file downloaded
/// from a game server, and a wrong offset here would corrupt a library that
/// the server needs in order to start.
pub fn find_gnu_stack(bytes: &[u8]) -> Result<Option<GnuStack>, String> {
    if bytes.get(..4) != Some(&ELF_MAGIC[..]) {
        return Err("not an ELF file".into());
    }
    match bytes.get(4) {
        Some(&CLASS_64) => {}
        Some(_) => return Err("not a 64-bit ELF".into()),
        None => return Err("truncated ELF header".into()),
    }
    if bytes.get(5) != Some(&DATA_LITTLE_ENDIAN) {
        return Err("not a little-endian ELF".into());
    }

    let phoff = u64_at(bytes, E_PHOFF).ok_or("truncated ELF header")? as usize;
    let phentsize = u16_at(bytes, E_PHENTSIZE).ok_or("truncated ELF header")? as usize;
    let phnum = u16_at(bytes, E_PHNUM).ok_or("truncated ELF header")? as usize;
    if phentsize < PH_ENTRY_MIN {
        return Err(format!("implausible program header size ({phentsize})"));
    }

    for index in 0..phnum {
        let entry = phoff
            .checked_add(index.checked_mul(phentsize).ok_or("program header table overflows")?)
            .ok_or("program header table overflows")?;
        // The whole entry must be inside the file, not just the fields read.
        if entry.checked_add(phentsize).is_none_or(|end| end > bytes.len()) {
            return Err("program header table runs past the end of the file".into());
        }
        if u32_at(bytes, entry).ok_or("truncated program header")? == PT_GNU_STACK {
            let flags_offset = entry + P_FLAGS;
            return Ok(Some(GnuStack {
                flags_offset,
                flags: u32_at(bytes, flags_offset).ok_or("truncated program header")?,
            }));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal ELF64 with one PT_GNU_STACK program header, laid out the way a
    /// real library is: header, then the program header table.
    fn elf_with_stack_flags(flags: u32) -> Vec<u8> {
        let phoff: u64 = 64;
        let mut bytes = vec![0u8; 64 + PH_ENTRY_MIN];
        bytes[..4].copy_from_slice(&ELF_MAGIC);
        bytes[4] = CLASS_64;
        bytes[5] = DATA_LITTLE_ENDIAN;
        bytes[E_PHOFF..E_PHOFF + 8].copy_from_slice(&phoff.to_le_bytes());
        bytes[E_PHENTSIZE..E_PHENTSIZE + 2].copy_from_slice(&(PH_ENTRY_MIN as u16).to_le_bytes());
        bytes[E_PHNUM..E_PHNUM + 2].copy_from_slice(&1u16.to_le_bytes());
        let entry = phoff as usize;
        bytes[entry..entry + 4].copy_from_slice(&PT_GNU_STACK.to_le_bytes());
        bytes[entry + P_FLAGS..entry + P_FLAGS + 4].copy_from_slice(&flags.to_le_bytes());
        bytes
    }

    #[test]
    fn finds_an_executable_stack_and_says_where_it_is() {
        // 7 = RWE, which is what CounterStrikeSharp ships.
        let bytes = elf_with_stack_flags(7);
        let found = find_gnu_stack(&bytes).expect("parses").expect("has PT_GNU_STACK");
        assert_eq!(found.flags, 7);
        assert!(found.executable());
        // The offset is what gets written to on the node, so it has to be the
        // p_flags field itself and not the start of the header.
        assert_eq!(found.flags_offset, 64 + P_FLAGS);
        assert_eq!(u32_at(&bytes, found.flags_offset), Some(7));
    }

    #[test]
    fn an_already_clear_flag_is_reported_as_clear() {
        let bytes = elf_with_stack_flags(6);
        let found = find_gnu_stack(&bytes).expect("parses").expect("has PT_GNU_STACK");
        assert_eq!(found.flags, 6);
        assert!(!found.executable(), "nothing may be written when there is nothing to fix");
    }

    #[test]
    fn a_library_without_the_header_is_not_an_error() {
        let mut bytes = elf_with_stack_flags(7);
        // Turn the only program header into something else.
        bytes[64..68].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(find_gnu_stack(&bytes).expect("parses"), None);
    }

    #[test]
    fn refuses_anything_that_is_not_a_little_endian_elf64() {
        assert!(find_gnu_stack(b"MZ not an elf").is_err());
        assert!(find_gnu_stack(&[]).is_err());

        let mut wrong_class = elf_with_stack_flags(7);
        wrong_class[4] = 1; // 32-bit
        assert!(find_gnu_stack(&wrong_class).is_err());

        let mut big_endian = elf_with_stack_flags(7);
        big_endian[5] = 2;
        assert!(find_gnu_stack(&big_endian).is_err());
    }

    /// A header table pointing past the end must be refused rather than read.
    #[test]
    fn refuses_a_header_table_outside_the_file() {
        // The lone header must not be PT_GNU_STACK, or the scan finds it at
        // index 0 and returns before it can reach the out-of-bounds entries.
        let mut bytes = elf_with_stack_flags(7);
        bytes[64..68].copy_from_slice(&1u32.to_le_bytes());
        bytes[E_PHNUM..E_PHNUM + 2].copy_from_slice(&64u16.to_le_bytes());
        assert!(find_gnu_stack(&bytes).is_err());

        let mut far = elf_with_stack_flags(7);
        far[E_PHOFF..E_PHOFF + 8].copy_from_slice(&u64::MAX.to_le_bytes());
        assert!(far.len() < usize::MAX);
        assert!(find_gnu_stack(&far).is_err());
    }
}
