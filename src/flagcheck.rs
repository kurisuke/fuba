/* fuba - Simulate football (soccer) match & tournament results.
 *
 * Copyright (C) 2018  Peter Helbing
 *
 * This program is free software; you can redistribute it and/or
 * modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation; either version 2
 * of the License, or (at your option) any later version.
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301, USA.
 *
 */

use std::iter::Peekable;

// BEGIN PUBLIC INTERFACE

#[derive(Clone)]
pub struct FlagCheck {
    node: ParseNode,
}

impl FlagCheck {
    pub fn new(input: &str) -> Result<FlagCheck, String> {
        match parse(input) {
            Ok(node) => Ok(FlagCheck { node }),
            Err(e) => Err(e),
        }
    }

    pub fn check(&self, flags: &[String]) -> Result<bool, String> {
        self.node.check(flags)
    }

    pub fn pretty_print(&self) -> String {
        self.node.pretty_print()
    }
}

// END PUBLIC INTERFACE

#[derive(Debug, Clone)]
enum GrammarItem {
    And,
    Or,
    Not,
    Flag(String),
    Paren,
}

#[derive(Debug, Clone)]
struct ParseNode {
    children: Vec<ParseNode>,
    entry: GrammarItem,
}

impl ParseNode {
    fn new(entry: GrammarItem) -> ParseNode {
        ParseNode {
            children: vec![],
            entry,
        }
    }

    fn check(&self, flags: &[String]) -> Result<bool, String> {
        match self.entry {
            GrammarItem::Paren => self.children.get(0).unwrap().check(flags),
            GrammarItem::Not => self
                .children
                .get(0)
                .unwrap()
                .check(flags)
                .and_then(|v| Ok(!v)),
            GrammarItem::And => {
                let r1 = self.children.get(0).unwrap().check(flags);
                let r2 = self.children.get(1).unwrap().check(flags);
                if let Ok(v1) = r1 {
                    if let Ok(v2) = r2 {
                        Ok(v1 && v2)
                    } else {
                        Err(String::from(
                            "Error in parse tree, missing lhs operand to AND",
                        ))
                    }
                } else {
                    Err(String::from(
                        "Error in parse tree, missing rhs operand to AND",
                    ))
                }
            }
            GrammarItem::Or => {
                let r1 = self.children.get(0).unwrap().check(flags);
                let r2 = self.children.get(1).unwrap().check(flags);
                if let Ok(v1) = r1 {
                    if let Ok(v2) = r2 {
                        Ok(v1 || v2)
                    } else {
                        Err(String::from("Error in parse tree, missing lhs to OR"))
                    }
                } else {
                    Err(String::from(
                        "Error in parse tree, missing rhs operand to OR",
                    ))
                }
            }
            GrammarItem::Flag(ref f) => match flags.iter().find(|&x| x == f) {
                Some(_) => Ok(true),
                None => Ok(false),
            },
        }
    }

    fn pretty_print(&self) -> String {
        match self.entry {
            GrammarItem::Paren => format!(
                "({})",
                self.children
                    .get(0)
                    .expect("paren needs one child")
                    .pretty_print()
            ),
            GrammarItem::Not => format!(
                "!{}",
                self.children
                    .get(0)
                    .expect("NOT needs one child")
                    .pretty_print()
            ),
            GrammarItem::Or => {
                let lhs = self
                    .children
                    .get(0)
                    .expect("OR needs two children")
                    .pretty_print();
                let rhs = self
                    .children
                    .get(1)
                    .expect("OR needs two children")
                    .pretty_print();
                format!("{} & {}", lhs, rhs)
            }
            GrammarItem::And => {
                let lhs = self
                    .children
                    .get(0)
                    .expect("AND needs two children")
                    .pretty_print();
                let rhs = self
                    .children
                    .get(1)
                    .expect("AND needs two children")
                    .pretty_print();
                format!("{} & {}", lhs, rhs)
            }
            GrammarItem::Flag(ref f) => f.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
enum LexItem {
    ParenOpen,
    ParenClose,
    Op(char),
    Flag(String),
}

fn lex(input: &str) -> Result<Vec<LexItem>, String> {
    let mut result = vec![];

    let mut it = input.chars().peekable();
    while let Some(&c) = it.peek() {
        match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' => {
                it.next();
                let f = get_flag(c, &mut it);
                result.push(LexItem::Flag(f));
            }
            '|' | '&' | '!' => {
                result.push(LexItem::Op(c));
                it.next();
            }
            '(' => {
                result.push(LexItem::ParenOpen);
                it.next();
            }
            ')' => {
                result.push(LexItem::ParenClose);
                it.next();
            }
            ' ' => {
                it.next();
            }
            _ => {
                return Err(format!("Unexpected character: {}", c));
            }
        }
    }

    Ok(result)
}

fn get_flag<T: Iterator<Item = char>>(c: char, it: &mut Peekable<T>) -> String {
    let mut flag = c.to_string();

    while let Some(&c) = it.peek() {
        if c.is_alphanumeric() {
            flag.push(c);
            it.next();
        } else {
            break;
        }
    }
    flag
}

fn parse(input: &str) -> Result<ParseNode, String> {
    let tokens = lex(input)?;

    parse_expr(&tokens, 0).and_then(|(n, i)| {
        if i == tokens.len() {
            Ok(n)
        } else {
            Err(format!(
                "Expected end of input, found {:?} at idx {}",
                tokens[i], i
            ))
        }
    })
}

fn parse_expr(tokens: &[LexItem], pos: usize) -> Result<(ParseNode, usize), String> {
    let (lhs, next_pos) = parse_or_operand(tokens, pos)?;
    let c = tokens.get(next_pos);
    match c {
        Some(&LexItem::Op('|')) => {
            let mut or_node = ParseNode::new(GrammarItem::Or);
            or_node.children.push(lhs);
            let (rhs, i) = parse_expr(tokens, next_pos + 1)?;
            or_node.children.push(rhs);
            Ok((or_node, i))
        }
        _ => Ok((lhs, next_pos)),
    }
}

fn parse_or_operand(tokens: &[LexItem], pos: usize) -> Result<(ParseNode, usize), String> {
    let (lhs, next_pos) = parse_and_operand(tokens, pos)?;
    let c = tokens.get(next_pos);
    match c {
        Some(&LexItem::Op('&')) => {
            let mut and_node = ParseNode::new(GrammarItem::And);
            and_node.children.push(lhs);
            let (rhs, i) = parse_expr(tokens, next_pos + 1)?;
            and_node.children.push(rhs);
            Ok((and_node, i))
        }
        _ => Ok((lhs, next_pos)),
    }
}

fn parse_and_operand(tokens: &[LexItem], pos: usize) -> Result<(ParseNode, usize), String> {
    let c: &LexItem = tokens
        .get(pos)
        .ok_or_else(|| String::from("Unexpected end of input, expected paren or number"))?;

    match *c {
        LexItem::Op('!') => {
            let mut not_node = ParseNode::new(GrammarItem::Not);
            let (operand, next_pos) = parse_expr(tokens, pos + 1)?;
            not_node.children.push(operand);
            Ok((not_node, next_pos))
        }
        LexItem::Flag(ref f) => {
            let node = ParseNode::new(GrammarItem::Flag(f.clone()));
            Ok((node, pos + 1))
        }
        LexItem::ParenOpen => parse_expr(tokens, pos + 1).and_then(|(node, next_pos)| {
            if let Some(&LexItem::ParenClose) = tokens.get(next_pos) {
                let mut paren = ParseNode::new(GrammarItem::Paren);
                paren.children.push(node);
                Ok((paren, next_pos + 1))
            } else {
                Err(format!(
                    "Expected closing paren at {} but found {:?}",
                    next_pos,
                    tokens.get(next_pos)
                ))
            }
        }),
        _ => Err(format!(
            "Unexpected token {:?}, expected paren or number",
            c
        )),
    }
}
