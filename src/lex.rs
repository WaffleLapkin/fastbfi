use Token::*;

/// All possible things that can be in a BF program.
#[derive(Debug, Copy, Clone)]
pub enum Token {
    /// `>`; Increment the data pointer by one (to point to the next cell to the right).
    RAngle,
    /// `<`; Decrement the data pointer by one (to point to the next cell to the left).
    LAngle,

    /// `+`; Increment the byte at the data pointer by one.
    Plus,
    /// `-`; Decrement the byte at the data pointer by one
    Minus,

    /// `.`; Output the byte at the data pointer.
    Dot,
    /// `,`; Accept one byte of input, storing its value in the byte at the data pointer.
    Comma,

    /// `[`
    LBrack,
    /// `]`
    RBrack,

    /// Anything else.
    Comment,

    /// End of file/input.
    Eof,
}

/// Creates a [`fn@Lexer`] which lexes the `source`.
#[allow(non_snake_case)]
pub fn Lexer(source: &str) -> Lexer<'_> {
    Lexer {
        source: source.bytes(),
    }
}

/// A lexer, duh.
pub struct Lexer<'a> {
    source: std::str::Bytes<'a>,
}

impl Lexer<'_> {
    /// Returns the next token.
    ///
    /// When end of file is reached, [`Eof`] token is returned.
    pub fn next(&mut self) -> Token {
        let Some(c) = self.source.next() else { return Token::Eof };

        match c {
            b'<' => LAngle,
            b'>' => RAngle,

            b'+' => Plus,
            b'-' => Minus,

            b'.' => Dot,
            b',' => Comma,

            b'[' => LBrack,
            b']' => RBrack,

            _ => Comment,
        }
    }

    /// Returns a *hint* on number of tokens returned by the lexer.
    pub fn len_hint(&self) -> usize {
        self.source.len()
    }
}
