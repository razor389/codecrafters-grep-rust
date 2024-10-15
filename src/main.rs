use std::env;
use std::io;
use std::process;

fn contains_digit(s: &str) -> bool {
    s.chars().any(|c| c.is_digit(10))  // Base 10 digits
}

fn cointains_alphanumeric(s: &str) -> bool{
    s.chars().any(|c| c.is_alphanumeric())
}

fn contains_specific_chars(s: &str, p: &str) -> bool{
    s.chars().any(|c| p.contains(c))
}

fn match_pattern(input_line: &str, pattern: &str) -> bool {
    if pattern.chars().count() == 1 {
        return input_line.contains(pattern);
    }
    else if pattern == "\\d"{
        return contains_digit(input_line);
    }
    else if pattern == "\\w"{
        return cointains_alphanumeric(input_line);
    }
    else if pattern.starts_with('[') && pattern.ends_with(']') && pattern.len() >2{
        return contains_specific_chars(input_line, &pattern[1..pattern.len()-1]);
    }
    else {
        panic!("Unhandled pattern: {}", pattern)
    }
}

// Usage: echo <input_text> | your_program.sh -E <pattern>
fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    if env::args().nth(1).unwrap() != "-E" {
        println!("Expected first argument to be '-E'");
        process::exit(1);
    }

    let pattern = env::args().nth(2).unwrap();
    let mut input_line = String::new();

    io::stdin().read_line(&mut input_line).unwrap();

    // Uncomment this block to pass the first stage
    if match_pattern(&input_line, &pattern) {
        process::exit(0)
    } else {
        process::exit(1)
    }
}
