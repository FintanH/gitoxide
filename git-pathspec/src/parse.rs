use bstr::{BString, ByteSlice};

use crate::{MagicSignature, Pattern, SearchMode};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Empty string is not a valid pathspec")]
    EmptyString,
    #[error("Found {:?}, which is not a valid keyword", found_keyword)]
    InvalidKeyword { found_keyword: BString },
    #[error("Unimplemented pathspec magic {:?}", found_short_keyword)]
    Unimplemented { found_short_keyword: char },
    #[error("Missing ')' at the end of pathspec magic in {:?}", pathspec)]
    MissingClosingParenthesis { pathspec: BString },
    #[error("Attribute has non-ascii characters or starts with '-': {:?}", attribute)]
    InvalidAttribute { attribute: BString },
    #[error("Attribute specification cannot be empty")]
    EmptyAttribute,
    #[error("'literal' and 'glob' keywords cannot be used together in the same pathspec")]
    IncompatibleSearchModes,
    #[error("Only one attribute specification is allowed in the same pathspec")]
    MultipleAttributeSpecifications,
}

impl Pattern {
    pub fn from_bytes(input: &[u8]) -> Result<Self, Error> {
        if input.is_empty() {
            return Err(Error::EmptyString);
        }

        let mut p = Pattern {
            path: BString::default(),
            signature: MagicSignature::empty(),
            search_mode: SearchMode::ShellGlob,
            attributes: Vec::new(),
        };

        let mut cursor = 0;
        if input.first() == Some(&b':') {
            cursor += 1;
            parse_short_keywords(input, &mut p, &mut cursor)?;
            if let Some(b'(') = input.get(cursor) {
                cursor += 1;
                parse_long_keywords(input, &mut p, &mut cursor)?;
            }
        }

        p.path = BString::from(&input[cursor..]);
        Ok(p)
    }
}

fn parse_short_keywords(input: &[u8], p: &mut Pattern, cursor: &mut usize) -> Result<(), Error> {
    let unimplemented_chars = vec![
        b'"', b'#', b'%', b'&', b'\'', b',', b'-', b';', b'<', b'=', b'>', b'@', b'_', b'`', b'~',
    ];

    while let Some(&b) = input.get(*cursor) {
        *cursor += 1;
        p.signature |= match b {
            b'/' => MagicSignature::TOP,
            b'^' | b'!' => MagicSignature::EXCLUDE,
            b':' => break,
            _ if unimplemented_chars.contains(&b) => {
                return Err(Error::Unimplemented {
                    found_short_keyword: b.into(),
                });
            }
            _ => {
                *cursor -= 1;
                break;
            }
        }
    }

    Ok(())
}

fn parse_long_keywords(input: &[u8], p: &mut Pattern, cursor: &mut usize) -> Result<(), Error> {
    let end = input.find(")").ok_or(Error::MissingClosingParenthesis {
        pathspec: BString::from(input),
    })?;

    let input = &input[*cursor..end];
    *cursor = end + 1;

    debug_assert_eq!(p.search_mode, SearchMode::default());

    if input.is_empty() {
        return Ok(());
    }

    for keyword in split_on_non_escaped_char(input, b',') {
        match keyword {
            b"attr" => continue,
            b"top" => p.signature |= MagicSignature::TOP,
            b"icase" => p.signature |= MagicSignature::ICASE,
            b"exclude" => p.signature |= MagicSignature::EXCLUDE,
            b"literal" => match p.search_mode {
                SearchMode::PathAwareGlob => return Err(Error::IncompatibleSearchModes),
                _ => p.search_mode = SearchMode::Literal,
            },
            b"glob" => match p.search_mode {
                SearchMode::Literal => return Err(Error::IncompatibleSearchModes),
                _ => p.search_mode = SearchMode::PathAwareGlob,
            },
            _ if keyword.starts_with(b"attr:") => {
                if p.attributes.is_empty() {
                    p.attributes = parse_attributes(&keyword[5..])?;
                } else {
                    return Err(Error::MultipleAttributeSpecifications);
                }
            }
            _ if keyword.starts_with(b"prefix:") => {
                // TODO: Needs research - what does 'prefix:' do
            }
            _ => {
                return Err(Error::InvalidKeyword {
                    found_keyword: BString::from(keyword),
                });
            }
        }
    }

    Ok(())
}

fn split_on_non_escaped_char(input: &[u8], split_char: u8) -> Vec<&[u8]> {
    let mut keywords = Vec::new();
    let mut i = 0;
    let mut last = 0;
    loop {
        if let Some(&b) = input.get(i + 1) {
            if b == split_char && input[i] != b'\\' {
                i += 1;
                keywords.push(&input[last..i]);
                last = i + 1;
            }
        } else {
            keywords.push(&input[last..]);
            break;
        }

        i += 1;
    }
    keywords
}

fn parse_attributes(input: &[u8]) -> Result<Vec<(BString, git_attributes::State)>, Error> {
    if input.is_empty() {
        return Err(Error::EmptyAttribute);
    }

    let unescaped = input.replace(r"\,", ",");

    git_attributes::parse::Iter::new(unescaped.as_bstr(), 0)
        .map(|res| res.map(|(name, state)| (name.into(), state.into())))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| match e {
            git_attributes::parse::Error::AttributeName {
                line_number: _,
                attribute,
            } => Error::InvalidAttribute { attribute },
            _ => unreachable!("expecting only 'Error::AttributeName' but got {}", e),
        })
}

fn _unescape_attribute_values(attrs: Vec<(BString, git_attributes::State)>) -> Vec<(BString, git_attributes::State)> {
    attrs
        .into_iter()
        .map(|(name, state)| {
            match &state {
                git_attributes::State::Value(_v) => {}
                _ => {}
            }
            (name, state)
        })
        .collect::<Vec<_>>()
}

fn _check_attr_value(value: &BString) -> Result<(), Error> {
    if value
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b',')
    {
        // Invalid character in value
        return Err(Error::InvalidAttribute {
            attribute: value.clone(),
        });
    }

    if value.ends_with(&[b'\\']) {
        // escape char '\' not allowed as last character
        return Err(Error::InvalidAttribute {
            attribute: value.clone(),
        });
    }

    Ok(())
}
