use std::{iter::Peekable, str::Chars};

#[derive(Clone)]
pub(crate) enum Token {
    Word(String),
    Redirect { fd: u32, append: bool },
    Pipe,
    Ampersand,
}

pub(crate) struct Lexer<'a> {
    input: Peekable<Chars<'a>>,
}

impl<'a> Lexer<'a> {
    pub(crate) fn new(input: &'a str) -> Self {
        Self {
            input: input.chars().peekable(),
        }
    }

    fn read_word(&mut self, first: char) -> String {
        let mut word = String::new();
        let mut current = Some(first);
        loop {
            match current {
                None => break,
                Some(ch) if ch.is_ascii_whitespace() => break,
                Some('\'') => self.read_single_quoted(&mut word),
                Some('"') => self.read_double_quoted(&mut word),
                Some('\\') => {
                    if let Some(ch) = self.input.next() {
                        word.push(ch);
                    }
                }
                Some(ch) => word.push(ch),
            }
            current = self.input.next();
        }
        word
    }

    fn read_single_quoted(&mut self, word: &mut String) {
        while let Some(ch) = self.input.next_if(|&c| c != '\'') {
            word.push(ch);
        }
        // consume closing `'`
        self.input.next();
    }

    fn read_double_quoted(&mut self, word: &mut String) {
        while let Some(ch) = self.input.next() {
            match ch {
                '"' => break,
                '\\' => match self.input.peek() {
                    Some('"' | '\\') => {
                        if let Some(c) = self.input.next() {
                            word.push(c);
                        }
                    }
                    _ => word.push('\\'),
                },
                ch => word.push(ch),
            }
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        while matches!(self.input.peek(), Some(c) if c.is_ascii_whitespace()) {
            self.input.next();
        }

        let token = match self.input.next()? {
            '>' => {
                let append = self.input.next_if_eq(&'>').is_some();
                Token::Redirect { fd: 1, append }
            }
            '|' => Token::Pipe,
            '&' => Token::Ampersand,
            ch if (ch == '1' || ch == '2') && self.input.peek() == Some(&'>') => {
                // consume '>'
                self.input.next();
                let append = self.input.next_if_eq(&'>').is_some();
                Token::Redirect {
                    fd: ch.to_digit(10).unwrap_or(1),
                    append,
                }
            }
            ch => Token::Word(self.read_word(ch)),
        };

        Some(token)
    }
}
