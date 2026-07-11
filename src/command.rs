use std::fs::{File, OpenOptions};

use crate::lex::{Lexer, Token};

pub struct Redirect {
    pub fd: Fd,
    pub target: String,
    pub append: bool,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Fd {
    Stdout,
    Stderr,
}

impl Redirect {
    pub fn open(&self) -> anyhow::Result<File> {
        OpenOptions::new()
            .write(true)
            .create(true)
            .append(self.append)
            .truncate(!self.append)
            .open(&self.target)
            .map_err(Into::into)
    }
}

pub(crate) enum StageEnd {
    Pipe,
    Background,
    End,
}
pub struct Command {
    pub name: String,
    pub args: Vec<String>,
    redirects: Vec<Redirect>,
}

impl Command {
    /// Parses one pipeline stage from input tokens. Stops consuming
    /// when it encounters a Pipe `|`, `>`, `&` or at the end of input.
    /// Returns the command plus whether a pipe followed it.
    pub(crate) fn parse_stage(tokens: &mut Lexer) -> Option<(Self, StageEnd)> {
        let name = match tokens.next()? {
            Token::Word(w) => w,
            // leading `>`, `|` or `&`
            Token::Redirect { .. } | Token::Pipe | Token::Ampersand => return None,
        };
        let mut args = Vec::new();
        let mut redirects = Vec::new();
        let mut end = StageEnd::End;

        while let Some(token) = tokens.next() {
            match token {
                Token::Word(w) => args.push(w),
                Token::Redirect { fd, append } => {
                    let fd = match fd {
                        1 => Fd::Stdout,
                        2 => Fd::Stderr,
                        _ => return None,
                    };
                    match &tokens.next() {
                        Some(Token::Word(target)) => redirects.push(Redirect {
                            fd,
                            target: target.clone(),
                            append,
                        }),
                        _ => return None,
                    }
                }
                Token::Pipe => {
                    end = StageEnd::Pipe;
                    break;
                }
                Token::Ampersand => {
                    end = StageEnd::Background;
                    break;
                }
            }
        }
        Some((
            Self {
                name,
                args,
                redirects,
            },
            end,
        ))
    }

    pub fn redirect(&self, fd: Fd) -> Option<&Redirect> {
        self.redirects.iter().find(|r| r.fd == fd)
    }
}

pub struct Pipeline {
    pub stages: Vec<Command>,
    pub background: bool,
}

impl Pipeline {
    pub fn parse(input: &str) -> Option<Self> {
        let mut tokens = Lexer::new(input);
        let mut stages = Vec::new();
        let mut background = false;

        loop {
            let (stage, end) = Command::parse_stage(&mut tokens)?;
            stages.push(stage);
            match end {
                StageEnd::Pipe => continue,
                StageEnd::Background => {
                    background = true;
                    if tokens.next().is_some() {
                        // trailing content after `&` is a syntax error
                        return None;
                    }
                    break;
                }
                StageEnd::End => break,
            }
        }
        Some(Self { stages, background })
    }
}
