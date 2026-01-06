//! NBT compound argument for commands like /summon.
//!
//! Parses SNBT (Stringified NBT) format like `{Health:20.0f,NoAI:1b}`.

use simdnbt::owned::{NbtCompound, NbtList, NbtTag};
use steel_protocol::packets::game::{ArgumentType, SuggestionType};

use crate::command::arguments::CommandArgument;
use crate::command::context::CommandContext;

/// An NBT compound argument that parses SNBT format.
pub struct NbtArgument;

impl CommandArgument for NbtArgument {
    type Output = NbtCompound;

    fn parse<'a>(
        &self,
        arg: &'a [&'a str],
        _context: &mut CommandContext,
    ) -> Option<(&'a [&'a str], Self::Output)> {
        // Join remaining args to get the full NBT string
        // NBT can contain spaces inside strings, so we need to handle this carefully
        let full_str = arg.join(" ");

        if !full_str.starts_with('{') {
            return None;
        }

        // Find the matching closing brace
        let (nbt_str, remaining_len) = extract_compound_string(&full_str)?;

        // Parse the SNBT string
        let compound = parse_snbt_compound(nbt_str)?;

        // Calculate how many args were consumed
        // This is approximate - we count tokens in what we consumed
        let consumed_len = nbt_str.len();
        let mut chars_counted = 0;
        let mut args_consumed = 0;

        for (i, a) in arg.iter().enumerate() {
            chars_counted += a.len();
            if i > 0 {
                chars_counted += 1; // space
            }
            args_consumed = i + 1;
            if chars_counted >= consumed_len {
                break;
            }
        }

        // If there's remaining content after the NBT, we need to figure out how many args to keep
        if remaining_len > 0 {
            // There's content after the NBT that wasn't consumed
            // For simplicity, assume NBT takes all remaining args
        }

        Some((&arg[args_consumed..], compound))
    }

    fn usage(&self) -> (ArgumentType, Option<SuggestionType>) {
        (ArgumentType::NbtTag, None)
    }
}

/// Extracts a compound string from the input, returning (`compound_str`, `remaining_len`)
fn extract_compound_string(input: &str) -> Option<(&str, usize)> {
    if !input.starts_with('{') {
        return None;
    }

    let mut depth = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for (i, c) in input.char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }

        match c {
            '\\' if in_string => escape_next = true,
            '"' => in_string = !in_string,
            '{' if !in_string => depth += 1,
            '}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some((&input[..=i], input.len() - i - 1));
                }
            }
            _ => {}
        }
    }

    None
}

/// Parses an SNBT compound string like `{Health:20.0f,NoAI:1b}`
fn parse_snbt_compound(input: &str) -> Option<NbtCompound> {
    let input = input.trim();
    if !input.starts_with('{') || !input.ends_with('}') {
        return None;
    }

    let inner = &input[1..input.len() - 1];
    let mut compound = NbtCompound::new();

    if inner.trim().is_empty() {
        return Some(compound);
    }

    // Parse key:value pairs
    let mut parser = SnbtParser::new(inner);

    while !parser.is_empty() {
        parser.skip_whitespace();

        if parser.is_empty() {
            break;
        }

        // Parse key
        let key = parser.parse_key()?;
        parser.skip_whitespace();

        // Expect colon
        if !parser.consume(':') {
            return None;
        }

        parser.skip_whitespace();

        // Parse value
        let value = parser.parse_value()?;

        compound.insert(key, value);

        parser.skip_whitespace();

        // Consume comma if present
        if !parser.consume(',') {
            break;
        }
    }

    Some(compound)
}

struct SnbtParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> SnbtParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn is_empty(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
    }

    fn consume(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.pos += expected.len_utf8();
            true
        } else {
            false
        }
    }

    fn parse_key(&mut self) -> Option<String> {
        if self.peek() == Some('"') {
            self.parse_quoted_string()
        } else {
            self.parse_unquoted_string()
        }
    }

    fn parse_quoted_string(&mut self) -> Option<String> {
        if !self.consume('"') {
            return None;
        }

        let mut result = String::new();
        let mut escape_next = false;

        while let Some(c) = self.peek() {
            self.pos += c.len_utf8();

            if escape_next {
                result.push(c);
                escape_next = false;
            } else if c == '\\' {
                escape_next = true;
            } else if c == '"' {
                return Some(result);
            } else {
                result.push(c);
            }
        }

        None
    }

    fn parse_unquoted_string(&mut self) -> Option<String> {
        let start = self.pos;

        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '+' {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }

        if self.pos > start {
            Some(self.input[start..self.pos].to_string())
        } else {
            None
        }
    }

    fn parse_value(&mut self) -> Option<NbtTag> {
        self.skip_whitespace();

        match self.peek()? {
            '{' => self.parse_compound(),
            '[' => self.parse_list_or_array(),
            '"' => self.parse_quoted_string().map(|s| NbtTag::String(s.into())),
            _ => self.parse_primitive(),
        }
    }

    fn parse_compound(&mut self) -> Option<NbtTag> {
        if !self.consume('{') {
            return None;
        }

        let mut compound = NbtCompound::new();

        self.skip_whitespace();

        if self.consume('}') {
            return Some(NbtTag::Compound(compound));
        }

        loop {
            self.skip_whitespace();

            let key = self.parse_key()?;
            self.skip_whitespace();

            if !self.consume(':') {
                return None;
            }

            self.skip_whitespace();
            let value = self.parse_value()?;

            compound.insert(key, value);

            self.skip_whitespace();

            if self.consume('}') {
                break;
            }

            if !self.consume(',') {
                return None;
            }
        }

        Some(NbtTag::Compound(compound))
    }

    fn parse_list_or_array(&mut self) -> Option<NbtTag> {
        if !self.consume('[') {
            return None;
        }

        self.skip_whitespace();

        // Check for typed arrays: [B;...], [I;...], [L;...]
        if let Some(c) = self.peek()
            && (c == 'B' || c == 'I' || c == 'L')
            && self.input[self.pos + 1..].starts_with(';')
        {
            let array_type = c;
            self.pos += 2; // Skip 'X;'
            return self.parse_typed_array(array_type);
        }

        // Regular list
        self.parse_list()
    }

    fn parse_typed_array(&mut self, array_type: char) -> Option<NbtTag> {
        let mut values = Vec::new();

        loop {
            self.skip_whitespace();

            if self.consume(']') {
                break;
            }

            if !values.is_empty() && !self.consume(',') {
                return None;
            }

            self.skip_whitespace();

            if self.consume(']') {
                break;
            }

            let value = self.parse_primitive()?;
            values.push(value);
        }

        match array_type {
            'B' => {
                let bytes: Vec<u8> = values
                    .into_iter()
                    .filter_map(|v| match v {
                        NbtTag::Byte(b) => Some(b as u8),
                        NbtTag::Int(i) => Some(i as u8),
                        _ => None,
                    })
                    .collect();
                Some(NbtTag::ByteArray(bytes))
            }
            'I' => {
                let ints: Vec<i32> = values
                    .into_iter()
                    .filter_map(|v| match v {
                        NbtTag::Int(i) => Some(i),
                        NbtTag::Byte(b) => Some(i32::from(b)),
                        _ => None,
                    })
                    .collect();
                Some(NbtTag::IntArray(ints))
            }
            'L' => {
                let longs: Vec<i64> = values
                    .into_iter()
                    .filter_map(|v| match v {
                        NbtTag::Long(l) => Some(l),
                        NbtTag::Int(i) => Some(i64::from(i)),
                        _ => None,
                    })
                    .collect();
                Some(NbtTag::LongArray(longs))
            }
            _ => None,
        }
    }

    fn parse_list(&mut self) -> Option<NbtTag> {
        let mut values = Vec::new();

        loop {
            self.skip_whitespace();

            if self.consume(']') {
                break;
            }

            if !values.is_empty() && !self.consume(',') {
                return None;
            }

            self.skip_whitespace();

            if self.consume(']') {
                break;
            }

            let value = self.parse_value()?;
            values.push(value);
        }

        // Convert to appropriate NbtList type based on contents
        if values.is_empty() {
            return Some(NbtTag::List(NbtList::Empty));
        }

        // Check first element type and create homogeneous list
        match &values[0] {
            NbtTag::Byte(_) => {
                let bytes: Vec<i8> = values
                    .into_iter()
                    .filter_map(|v| match v {
                        NbtTag::Byte(b) => Some(b),
                        _ => None,
                    })
                    .collect();
                Some(NbtTag::List(NbtList::Byte(bytes)))
            }
            NbtTag::Int(_) => {
                let ints: Vec<i32> = values
                    .into_iter()
                    .filter_map(|v| match v {
                        NbtTag::Int(i) => Some(i),
                        _ => None,
                    })
                    .collect();
                Some(NbtTag::List(NbtList::Int(ints)))
            }
            NbtTag::Long(_) => {
                let longs: Vec<i64> = values
                    .into_iter()
                    .filter_map(|v| match v {
                        NbtTag::Long(l) => Some(l),
                        _ => None,
                    })
                    .collect();
                Some(NbtTag::List(NbtList::Long(longs)))
            }
            NbtTag::Float(_) => {
                let floats: Vec<f32> = values
                    .into_iter()
                    .filter_map(|v| match v {
                        NbtTag::Float(f) => Some(f),
                        _ => None,
                    })
                    .collect();
                Some(NbtTag::List(NbtList::Float(floats)))
            }
            NbtTag::Double(_) => {
                let doubles: Vec<f64> = values
                    .into_iter()
                    .filter_map(|v| match v {
                        NbtTag::Double(d) => Some(d),
                        _ => None,
                    })
                    .collect();
                Some(NbtTag::List(NbtList::Double(doubles)))
            }
            NbtTag::String(_) => {
                let strings: Vec<simdnbt::Mutf8String> = values
                    .into_iter()
                    .filter_map(|v| match v {
                        NbtTag::String(s) => Some(s),
                        _ => None,
                    })
                    .collect();
                Some(NbtTag::List(NbtList::String(strings)))
            }
            NbtTag::Compound(_) => {
                let compounds: Vec<NbtCompound> = values
                    .into_iter()
                    .filter_map(|v| match v {
                        NbtTag::Compound(c) => Some(c),
                        _ => None,
                    })
                    .collect();
                Some(NbtTag::List(NbtList::Compound(compounds)))
            }
            _ => Some(NbtTag::List(NbtList::Empty)),
        }
    }

    fn parse_primitive(&mut self) -> Option<NbtTag> {
        let start = self.pos;

        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '+' {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }

        if self.pos == start {
            return None;
        }

        let token = &self.input[start..self.pos];

        let lower = token.to_lowercase();

        if lower == "true" {
            return Some(NbtTag::Byte(1));
        }
        if lower == "false" {
            return Some(NbtTag::Byte(0));
        }

        if lower.ends_with('b')
            && let Ok(v) = token[..token.len() - 1].parse::<i8>()
        {
            return Some(NbtTag::Byte(v));
        }

        if lower.ends_with('s')
            && let Ok(v) = token[..token.len() - 1].parse::<i16>()
        {
            return Some(NbtTag::Short(v));
        }

        if lower.ends_with('l')
            && let Ok(v) = token[..token.len() - 1].parse::<i64>()
        {
            return Some(NbtTag::Long(v));
        }

        if lower.ends_with('f')
            && let Ok(v) = token[..token.len() - 1].parse::<f32>()
        {
            return Some(NbtTag::Float(v));
        }

        if lower.ends_with('d')
            && let Ok(v) = token[..token.len() - 1].parse::<f64>()
        {
            return Some(NbtTag::Double(v));
        }

        if token.contains('.')
            && let Ok(v) = token.parse::<f64>()
        {
            return Some(NbtTag::Double(v));
        }

        if let Ok(v) = token.parse::<i32>() {
            return Some(NbtTag::Int(v));
        }

        Some(NbtTag::String(token.into()))
    }
}
