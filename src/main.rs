use std::collections::HashMap;
use std::env;
use std::process;

#[derive(Debug, Clone, PartialEq)]
enum RE {
    Char(char),                 // A literal character
    Question(Box<RE>),          // A character or regex type followed by '?'
    Plus(Box<RE>),              // A character or regex type followed by '+'
    Dot,                        // The '.' metacharacter
    Start,                      // The '^' metacharacter
    End,                        // The '$' metacharacter
    CharClass(Vec<char>),       // A character class, e.g., [a-z]
    NegCharClass(Vec<char>),    // A negated character class, e.g., [^a-z]
    Digit,                      // Shorthand for \d (any digit)
    Word,                       // Shorthand for \w (alphanumeric character)
    Alternation(Vec<RE>, Vec<RE>), // Alternation between two patterns, e.g., (cat|dog)
    Group(Vec<RE>),             // A grouped sub-pattern, e.g., (cat)
    Backreference(usize),       // A backreference to a previously captured group, e.g., \1
}

struct RegexEngine {
    pattern: Vec<RE>,
    captures: HashMap<usize, String>, // Captured groups stored by their index
    //group_index: usize,
}

impl RegexEngine {
    fn new(pattern: &str) -> Self {
        let parsed_pattern = parse_pattern(pattern);
        println!("pattern: {:?}", parsed_pattern);
        Self {
            pattern: parsed_pattern,
            captures: HashMap::new(),
            //group_index: 0,
        }
    }

    fn match_text(&mut self, text: &str) -> bool {
        let pattern = self.pattern.clone();
        self.captures.clear();
        //self.group_index = 0; // Reset group index for each new match attempt
        self.match_pattern(&pattern, text, 0)
    }

    fn match_pattern(&mut self, pattern: &[RE], text: &str, mut local_group_index: usize) -> bool {
        if let Some(RE::Start) = pattern.get(0) {
            self.match_here(&pattern[1..], text, local_group_index)
        } else {
            let mut text_slice = text;
            loop {
                if self.match_here(pattern, text_slice, local_group_index) {
                    return true;
                }
                if text_slice.is_empty() {
                    break;
                }
                text_slice = &text_slice[1..];
            }
            false
        }
    }

    fn match_here(&mut self, pattern: &[RE], text: &str, mut local_group_index: usize) -> bool {
        if pattern.is_empty() {
            return true;
        }

        match &pattern[0] {
            RE::End => text.is_empty(),
            RE::Char(c) => {
                if !text.is_empty() && text.chars().next() == Some(*c) {
                    self.match_here(&pattern[1..], &text[1..], local_group_index)
                } else {
                    false
                }
            }
            RE::Dot => {
                if !text.is_empty() {
                    self.match_here(&pattern[1..], &text[1..], local_group_index)
                } else {
                    false
                }
            }
            RE::Question(boxed_re) => self.match_question(&**boxed_re, &pattern[1..], text, local_group_index),
            RE::Plus(boxed_re) => self.match_plus(&**boxed_re, &pattern[1..], text, local_group_index),
            RE::CharClass(class) => {
                if !text.is_empty() && class.contains(&text.chars().next().unwrap()) {
                    self.match_here(&pattern[1..], &text[1..], local_group_index)
                } else {
                    false
                }
            }
            RE::NegCharClass(class) => {
                if !text.is_empty() && !class.contains(&text.chars().next().unwrap()) {
                    self.match_here(&pattern[1..], &text[1..], local_group_index)
                } else {
                    false
                }
            }
            RE::Digit => {
                if !text.is_empty() && text.chars().next().unwrap().is_ascii_digit() {
                    self.match_here(&pattern[1..], &text[1..], local_group_index)
                } else {
                    false
                }
            }
            RE::Word => {
                if !text.is_empty() && text.chars().next().unwrap().is_alphanumeric() {
                    self.match_here(&pattern[1..], &text[1..], local_group_index)
                } else {
                    false
                }
            }
            RE::Backreference(group_index) => {
                println!("trying to capture: {}", group_index);
                println!("available captures: {:#?}", self.captures);
                if let Some(captured) = self.captures.get(group_index) {
                    if text.starts_with(captured) {
                        self.match_here(&pattern[1..], &text[captured.len()..], local_group_index)
                    } else {
                        false
                    }
                } else {
                    false // No capture for this group yet
                }
            }
            RE::Group(group_pattern) => {
                // Save the current state of group_index and captures
                let original_captures = self.captures.clone();
                let original_group_index = local_group_index;

                // Increment local_group_index for this group
                local_group_index += 1;
                let group_index = local_group_index;

                // Attempt to match the group at different lengths
                for len in 0..=text.len() {
                    let slice = &text[..len];
                    self.captures = original_captures.clone(); // Restore captures before each attempt

                    if self.match_pattern(group_pattern, slice, local_group_index) {
                        // Only assign the capture if the group matches
                        self.captures.insert(group_index, slice.to_string());

                        if self.match_here(&pattern[1..], &text[len..], local_group_index) {
                            return true;
                        }
                    }
                }

                // Restore state if no match is found
                self.captures = original_captures;
                local_group_index = original_group_index;

                false
            }
            RE::Alternation(left, right) => {
                let original_captures = self.captures.clone();
                let original_group_index = local_group_index;

                // Try matching the left side of the alternation
                if self.match_pattern(&left, text, local_group_index) {
                    return true;
                }

                // Restore state if the left branch fails
                self.captures = original_captures.clone();
                local_group_index = original_group_index;

                // Try matching the right side of the alternation
                if self.match_pattern(&right, text, local_group_index) {
                    return true;
                }

                // Restore state if both branches fail
                self.captures = original_captures;
                local_group_index = original_group_index;

                false
            }
            _ => false,
        }
    }

    fn match_question(&mut self, re: &RE, pattern: &[RE], text: &str, local_group_index: usize) -> bool {
        if self.match_here(pattern, text, local_group_index) {
            return true;
        }

        if !text.is_empty() && self.matches_char(re, text.chars().next().unwrap()) {
            self.match_here(pattern, &text[1..], local_group_index)
        } else {
            false
        }
    }

    fn match_plus(&mut self, re: &RE, pattern: &[RE], text: &str, mut local_group_index: usize) -> bool {
        let mut text_slice = text;

        if !text_slice.is_empty() && self.matches_char(re, text_slice.chars().next().unwrap()) {
            text_slice = &text_slice[1..];
        } else {
            return false;
        }

        loop {
            if self.match_here(pattern, text_slice, local_group_index) {
                return true;
            }
            if text_slice.is_empty() || !self.matches_char(re, text_slice.chars().next().unwrap()) {
                break;
            }
            text_slice = &text_slice[1..];
        }
        false
    }
    

    fn matches_char(&self, re: &RE, c: char) -> bool {
        match re {
            RE::Char(ch) => *ch == c,
            RE::Dot => true,
            RE::Digit => c.is_ascii_digit(),
            RE::Word => c.is_alphanumeric(),
            RE::CharClass(class) => class.contains(&c),
            RE::NegCharClass(class) => !class.contains(&c),
            _ => false,
        }
    }
}

fn parse_pattern(pattern: &str) -> Vec<RE> {
    let mut result = Vec::new();
    let chars: Vec<char> = pattern.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            '^' => result.push(RE::Start),
            '$' => result.push(RE::End),
            '.' => result.push(RE::Dot),
            '\\' => {
                if i + 1 < chars.len() {
                    match chars[i + 1] {
                        'd' => result.push(RE::Digit),
                        'w' => result.push(RE::Word),
                        '1'..='9' => {
                            let group_index = chars[i + 1].to_digit(10).unwrap() as usize;
                            result.push(RE::Backreference(group_index));
                        }
                        '\\' => result.push(RE::Char('\\')),
                        _ => panic!("Unsupported escape sequence: \\{}", chars[i + 1]),
                    }
                    i += 1;
                } else {
                    panic!("Pattern ends with an incomplete escape sequence");
                }
            }
            '[' => {
                if i + 1 < chars.len() && chars[i + 1] == '^' {
                    let (class, end_idx) = parse_char_class(&chars, i + 2);
                    result.push(RE::NegCharClass(class));
                    i = end_idx;
                } else {
                    let (class, end_idx) = parse_char_class(&chars, i + 1);
                    result.push(RE::CharClass(class));
                    i = end_idx;
                }
            }
            '(' => {
                // Check if the previous element is RE::Start
                let prepend_start = result.last() == Some(&RE::Start);

                let (mut group, end_idx) = parse_alternation(&chars, i + 1);
                // If we should prepend RE::Start, do so
                if prepend_start{
                    if let RE::Group(mut group_inner) = group{
                        group_inner.insert(0, RE::Start);
                        group = RE::Group(group_inner);
                    }
                }
                
                result.push(group);
                i = end_idx;
            }
            '?' => {
                if let Some(last) = result.pop() {
                    result.push(RE::Question(Box::new(last)));
                } else {
                    panic!("Invalid pattern: '?' cannot be the first character");
                }
            }
            '+' => {
                if let Some(last) = result.pop() {
                    result.push(RE::Plus(Box::new(last)));
                } else {
                    panic!("Invalid pattern: '+' cannot be the first character");
                }
            }
            ch => result.push(RE::Char(ch)),
        }
        i += 1;
    }
    //println!("pattern to match: {:#?}", result);
    result
}

fn parse_sequence(chars: &[char], i: &mut usize) -> Vec<RE> {
    let mut result = Vec::new();

    while *i < chars.len() {
        match chars[*i] {
            '|' | ')' => break, // Stop when encountering alternation or end of group
            '^' => result.push(RE::Start),
            '$' => result.push(RE::End),
            '.' => result.push(RE::Dot),
            '\\' => {
                if *i + 1 < chars.len() {
                    match chars[*i + 1] {
                        'd' => result.push(RE::Digit),
                        'w' => result.push(RE::Word),
                        '1'..='9' => {
                            let group_index = chars[*i + 1].to_digit(10).unwrap() as usize;
                            result.push(RE::Backreference(group_index));
                        }
                        '\\' => result.push(RE::Char('\\')),
                        _ => panic!("Unsupported escape sequence: \\{}", chars[*i + 1]),
                    }
                    *i += 1;
                } else {
                    panic!("Pattern ends with an incomplete escape sequence");
                }
            }
            '[' => {
                if *i + 1 < chars.len() && chars[*i + 1] == '^' {
                    let (class, end_idx) = parse_char_class(chars, *i + 2);
                    result.push(RE::NegCharClass(class));
                    *i = end_idx;
                } else {
                    let (class, end_idx) = parse_char_class(chars, *i + 1);
                    result.push(RE::CharClass(class));
                    *i = end_idx;
                }
            }
            '(' => {
                *i += 1; // Move past '('
                let (group, end_idx) = parse_alternation(chars, *i);
                result.push(group);
                *i = end_idx;
            }
            '?' => {
                if let Some(last) = result.pop() {
                    result.push(RE::Question(Box::new(last)));
                } else {
                    panic!("Invalid pattern: '?' cannot be the first character");
                }
            }
            '+' => {
                if let Some(last) = result.pop() {
                    result.push(RE::Plus(Box::new(last)));
                } else {
                    panic!("Invalid pattern: '+' cannot be the first character");
                }
            }
            ch => result.push(RE::Char(ch)),
        }
        *i += 1;
    }

    result
}

fn parse_alternation(chars: &[char], start: usize) -> (RE, usize) {
    let mut i = start;
    let left_side = parse_sequence(chars, &mut i);
    if i < chars.len() && chars[i] == '|' {
        i += 1; // Move past '|'
        let right_side = parse_sequence(chars, &mut i);
        if i < chars.len() && chars[i] == ')' {
            // Wrap the alternation in a group directly
            return (
                RE::Group(vec![RE::Alternation(left_side, right_side)]),
                i,
            );
        } else {
            panic!("Unmatched parenthesis or incomplete alternation");
        }
    } else if i < chars.len() && chars[i] == ')' {
        return (RE::Group(left_side), i);
    } else {
        panic!("Unmatched parenthesis or invalid alternation syntax");
    }
}


fn parse_char_class(chars: &[char], start: usize) -> (Vec<char>, usize) {
    let mut class = Vec::new();
    let mut i = start;

    while i < chars.len() {
        if chars[i] == ']' {
            return (class, i);
        } else if i + 2 < chars.len() && chars[i + 1] == '-' && chars[i + 2] != ']' {
            // Handle range like a-z
            let start = chars[i];
            let end = chars[i + 2];
            if start <= end {
                for c in start..=end {
                    class.push(c);
                }
            }
            i += 2;
        } else {
            class.push(chars[i]);
        }
        i += 1;
    }

    panic!("Unterminated character class");
}

// Usage: echo <input_text> | your_program.sh -E <pattern>
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 || args[1] != "-E" {
        eprintln!("Usage: your_program -E <pattern>");
        process::exit(1);
    }

    let pattern_str = &args[2];
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).expect("Failed to read input");
    let input = input.trim();

    let mut engine = RegexEngine::new(pattern_str);
    if engine.match_text(input) {
        process::exit(0);
    } else {
        process::exit(1);
    }
}
