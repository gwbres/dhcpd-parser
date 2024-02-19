use crate::leases::parse_lease;
use crate::leases::Lease;
use crate::leases::Leases;
pub use crate::leases::LeasesMethods;
use crate::lex::lex;
use crate::lex::LexItem;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserResult {
    pub leases: Leases,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigKeyword {
    Lease,
    Comment,
}

impl ConfigKeyword {
    pub fn to_string(&self) -> String {
        match self {
            &Self::Lease => "lease".to_owned(),
            &Self::Comment => "#".to_owned(),
        }
    }

    pub fn from(s: &str) -> Result<Self, String> {
        if s.starts_with('#') {
            Ok(Self::Comment)
        } else {
            match s {
                "lease" => Ok(Self::Lease),
                _ => Err(format!("'{}' declaration is not supported", s)),
            }
        }
    }
}

fn parse_config(tokens: Vec<LexItem>) -> Result<ParserResult, String> {
    let mut leases = Leases::new();
    let lease = Lease::new();

    let mut it = tokens.iter().peekable();

    while let Some(token) = it.peek() {
        match token {
            LexItem::Decl(ConfigKeyword::Comment) => {}
            LexItem::Decl(ConfigKeyword::Lease) => {
                if lease != Lease::new() {
                    leases.push(lease.clone());
                }

                let mut lease = Lease::new();
                // ip-address
                it.next();
                lease.ip = it.peek().expect("IP address expected").to_string();

                // left curly brace
                it.next();
                assert_eq!(it.peek().unwrap().to_owned(), &LexItem::Paren('{'));

                // statements for the lease
                it.next();
                parse_lease(&mut lease, &mut it)?;

                // right curly brace
                if it.peek().is_none() || it.peek().unwrap().to_owned() != &LexItem::Paren('}') {
                    return Err(format!(
                        "Expected end of section with '}}', got '{:?}'",
                        it.peek(),
                    ));
                }

                leases.push(lease.clone());
                it.next();
            }
            _ => {
                return Err(format!("Unexpected {:?}", it.peek()));
            }
        }
    }

    Ok(ParserResult { leases: leases })
}

pub fn parse<S>(input: S) -> Result<ParserResult, String>
where
    S: Into<String>,
{
    let tokens = lex(input).unwrap();
    return parse_config(tokens);
}
