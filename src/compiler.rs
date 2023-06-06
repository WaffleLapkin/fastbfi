use std::mem::size_of;

use crate::lex::Lexer;

/// The address type, which can represent where jumps jump to.
pub type Addr = u32;

/// Size of [`Addr`] in bytes.
pub const ADDR_SIZE: usize = size_of::<Addr>();

/// This a representation of a BF instruction.
///
/// This mostly just follows the "specification" of the language itself, with two notable differences:
/// - Instructions for `[` and `]` store the address where they jump to (which also means they are
///   the only 5 byte instructions)
///   - Note that this enum is data-less, to get the address you need to read byte code after those
///     instructions
/// - A "halt" instruction is added as a representation of EOF.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(usize)]
pub enum Inst {
    /// Increment the data pointer by one (to point to the next cell to the "right").
    IncPtr,

    /// `<`; Decrement the data pointer by one (to point to the next cell to the "left").
    DecPtr,

    /// `+`; Increment the byte at the data pointer by one.
    Inc,

    /// `-`; Decrement the byte at the data pointer by one
    Dec,

    /// `.`; Output the byte at the data pointer.
    Out,

    /// `,`; Accept one byte of input, storing its value in the byte at the data pointer.
    Inp,

    /// Jump if zero.
    ///
    /// This is a special instruction, as it is encoded with a [`ADDR_SIZE`] byte address after it (LE).
    ///
    /// This is how `[` is "desugared".
    ///
    /// If the byte at the data pointer is zero, then instead of moving the instruction pointer
    /// forward to the next command, jump it (forward) to the address encoded right after the
    /// instruction. For example:
    ///
    /// ```ignore (illustrative)
    /// if bc[i] is Inst::Jz && *data_ptr == 0 {
    ///     let addr = Addr::from_le_bytes(bc[i..][1..][..ADDR_SIZE]);
    ///     jump(addr);
    /// }
    /// ```
    Jz,

    /// Jump if not zero.
    ///
    /// This is a special instruction, as it is encoded with a [`ADDR_SIZE`] byte address after it (LE).
    ///
    /// This is how `]` is "desugared".
    ///
    /// If the byte at the data pointer is not zero, then instead of moving the instruction pointer
    /// forward to the next command, jump it (backwards) to the address encoded right after the
    /// instruction. For example:
    ///
    /// ```ignore (illustrative)
    /// if bc[i] is Inst::Jz && *data_ptr != 0 {
    ///     let addr = Addr::from_le_bytes(bc[i..][1..][..ADDR_SIZE]);
    ///     jump(addr);
    /// }
    /// ```
    Jnz,

    /// It's time to stop, ok?
    Halt,
}

/// Converts lexer output into byte-code.
///
/// Returns `Err` if `[`s and `]`s are not matched properly in the source.
pub fn compile(mut source: Lexer) -> Result<Vec<u8>, ()> {
    let mut v = Vec::with_capacity(source.len_hint());
    let mut jump_stack = Vec::with_capacity(4);

    loop {
        use crate::lex::Token::*;

        let t = source.next();

        match t {
            // Simple instructions that just record their representation
            RAngle => v.push(Inst::IncPtr.to_bc()),
            LAngle => v.push(Inst::DecPtr.to_bc()),

            Plus => v.push(Inst::Inc.to_bc()),
            Minus => v.push(Inst::Dec.to_bc()),

            Dot => v.push(Inst::Out.to_bc()),
            Comma => v.push(Inst::Inp.to_bc()),

            // More 'fun' stuff
            LBrack => {
                // Record the jump if zero instruction itself
                v.push(Inst::Jz.to_bc());

                // Record the address where to jump; since we don't yet know where the matching `]`
                // is, we just push a temporary garbage.
                v.extend([42; ADDR_SIZE]);

                // Push the addr where the matching `]` should jump to (after the addr encoding):
                //
                // [... Jz, 42, 42, 42, 42, ø]
                //                          ^-- this is what we store
                //                              in the jump stack
                jump_stack.push(v.len());
            }

            RBrack => {
                // Record the jump if non zero instruction itself
                v.push(Inst::Jnz.to_bc());

                // Find the matching `[`
                let Some(jump_addr) = jump_stack.pop() else { break Err(()) };

                // Record the address where this `]` will jump to
                let jump_there = Addr::try_from(jump_addr).map_err(drop)?.to_le_bytes();
                v.extend(jump_there);

                // Patch the address where the matching `[` jumps to.
                // Note that we use `jump_addr - ADDR_SIZE`,
                // because `jump_addr` is *after* the address.
                let jump_here = Addr::try_from(v.len()).map_err(drop)?.to_le_bytes();
                v[jump_addr - ADDR_SIZE..][..ADDR_SIZE].copy_from_slice(&jump_here);

                // After this we have the following picture:
                //
                //                     *--------------------------*
                //            ________/                           |
                //           /        \                           v
                // [..., Jz, a, b, c, d, N, ..., Jnz, x, y, z, w, ø]
                //                       ^            \________/
                //                       |            /
                //                       *-----------*
                //
                // Where [a, b, c, d] is the LE representation on Jz's jumping addr;
                // Where [x, y, z, w] is the LE representation on Jnz's jumping addr;
                // N is the instruction following `Jz`.
                // (it could possibly be Jnz, which would make a `if data is not 0 { loop{} }`)
            }

            Comment => continue,

            // The program has ended, but we still haven't found a match for some `[`,
            // this is not a valid BF program.
            Eof if !jump_stack.is_empty() => {
                break Err(());
            }

            Eof => {
                // Add a halt instruction to the end of the program, this maybe probably helps
                // avoid the bounds checks.
                v.push(Inst::Halt.to_bc());

                // Shrink the byte code vec, because I feel like it.
                v.shrink_to_fit();

                // Compilation succeeded :thumbeline:
                break Ok(v);
            }
        }
    }
}

impl Inst {
    /// Converts this instruction to the byte-code representation.
    ///
    /// This is the inversion of `from_bc`.
    pub const fn to_bc(self) -> u8 {
        self as usize as u8
    }

    /// Constructs an instruction this instruction to the byte-code representation.
    ///
    /// Return `None` if this is not a valid representation of an instruction.
    ///
    /// This is the inversion of `to_bc`.
    pub const fn from_bc(x: u8) -> Option<Self> {
        #![allow(non_upper_case_globals)]

        const IncPtr: u8 = Inst::IncPtr.to_bc();
        const DecPtr: u8 = Inst::DecPtr.to_bc();
        const Inc: u8 = Inst::Inc.to_bc();
        const Dec: u8 = Inst::Dec.to_bc();
        const Out: u8 = Inst::Out.to_bc();
        const Inp: u8 = Inst::Inp.to_bc();
        const Jf: u8 = Inst::Jz.to_bc();
        const Jb: u8 = Inst::Jnz.to_bc();
        const Halt: u8 = Inst::Halt.to_bc();

        let inst = match x {
            IncPtr => Inst::IncPtr,
            DecPtr => Inst::DecPtr,
            Inc => Inst::Inc,
            Dec => Inst::Dec,
            Out => Inst::Out,
            Inp => Inst::Inp,
            Jf => Inst::Jz,
            Jb => Inst::Jnz,
            Halt => Inst::Halt,
            _ => return None,
        };

        Some(inst)
    }
}
