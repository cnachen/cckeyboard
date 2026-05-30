const MORSE_UNIT_MS: u32 = 200;
pub const DOT_DASH_THRESHOLD_MS: u32 = MORSE_UNIT_MS * 2;
pub const LETTER_GAP_MS: u32 = MORSE_UNIT_MS * 3;
pub const WORD_GAP_MS: u32 = MORSE_UNIT_MS * 7;

#[derive(Clone, Copy)]
pub enum Symbol {
    Dot,
    Dash,
}

pub struct MorseDecoder {
    len: u8,
    bits: u8,
}

impl MorseDecoder {
    pub const fn new() -> Self {
        Self { len: 0, bits: 0 }
    }

    pub fn push(&mut self, symbol: Symbol) {
        if self.len >= 5 {
            self.clear();
            return;
        }

        self.bits <<= 1;
        if let Symbol::Dash = symbol {
            self.bits |= 1;
        }
        self.len += 1;
    }

    pub fn has_symbols(&self) -> bool {
        self.len != 0
    }

    pub fn take_char(&mut self) -> Option<char> {
        let decoded = decode_morse(self.len, self.bits);
        self.clear();
        decoded
    }

    fn clear(&mut self) {
        self.len = 0;
        self.bits = 0;
    }
}

fn decode_morse(len: u8, bits: u8) -> Option<char> {
    match (len, bits) {
        (1, 0b0) => Some('E'),
        (1, 0b1) => Some('T'),
        (2, 0b00) => Some('I'),
        (2, 0b01) => Some('A'),
        (2, 0b10) => Some('N'),
        (2, 0b11) => Some('M'),
        (3, 0b000) => Some('S'),
        (3, 0b001) => Some('U'),
        (3, 0b010) => Some('R'),
        (3, 0b011) => Some('W'),
        (3, 0b100) => Some('D'),
        (3, 0b101) => Some('K'),
        (3, 0b110) => Some('G'),
        (3, 0b111) => Some('O'),
        (4, 0b0000) => Some('H'),
        (4, 0b0001) => Some('V'),
        (4, 0b0010) => Some('F'),
        (4, 0b0100) => Some('L'),
        (4, 0b0110) => Some('P'),
        (4, 0b0111) => Some('J'),
        (4, 0b1000) => Some('B'),
        (4, 0b1001) => Some('X'),
        (4, 0b1010) => Some('C'),
        (4, 0b1011) => Some('Y'),
        (4, 0b1100) => Some('Z'),
        (4, 0b1101) => Some('Q'),
        (5, 0b00000) => Some('5'),
        (5, 0b00001) => Some('4'),
        (5, 0b00011) => Some('3'),
        (5, 0b00111) => Some('2'),
        (5, 0b01111) => Some('1'),
        (5, 0b11111) => Some('0'),
        (5, 0b11110) => Some('9'),
        (5, 0b11100) => Some('8'),
        (5, 0b11000) => Some('7'),
        (5, 0b10000) => Some('6'),
        _ => None,
    }
}
