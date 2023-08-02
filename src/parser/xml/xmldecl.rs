use crate::intmuttree::XMLDecl;
use crate::parser::combinators::map::map;
use crate::parser::combinators::opt::opt;
use crate::parser::combinators::tag::tag;
use crate::parser::combinators::tuple::{tuple5, tuple6, tuple8};
use crate::parser::combinators::wellformed::wellformed;
use crate::parser::combinators::whitespace::{whitespace0, whitespace1};
use crate::parser::xml::strings::delimited_string;
use crate::parser::{ParseError, ParseInput, ParseResult};

fn xmldeclversion() -> impl Fn(ParseInput) -> ParseResult<String> {
    move |input| match tuple5(
        tag("version"),
        whitespace0(),
        tag("="),
        whitespace0(),
        delimited_string(),
    )(input)
    {
        Ok((input1, (_, _, _, _, v))) => {
            if v == *"1.1" {
                Ok((input1, v))
            } else if v.starts_with("1.") {
                Ok((input1, "1.0".to_string()))
            } else {
                Err(ParseError::Notimplemented)
            }
        }
        Err(err) => Err(err),
    }
}

fn xmldeclstandalone() -> impl Fn(ParseInput) -> ParseResult<String> {
    move |(input, state)| {
        match map(
            wellformed(
                tuple6(
                    whitespace1(),
                    tag("standalone"),
                    whitespace0(),
                    tag("="),
                    whitespace0(),
                    delimited_string(),
                ),
                |(_, _, _, _, _, s)| vec!["yes".to_string(), "no".to_string()].contains(s),
            ),
            |(_, _, _, _, _, s)| s,
        )((input, state)){
            Err(e) => {Err(e) }
            Ok(((input2, mut state2), sta)) => {
                if &sta == "yes" {
                    state2.standalone = true;
                }
                Ok(((input2, state2),sta))
            }
        }
    }
}

pub(crate) fn xmldecl() -> impl Fn(ParseInput) -> ParseResult<XMLDecl> {
    move |(input, state)| {
        match
        tuple8(
            tag("<?xml"),
            whitespace1(),
            xmldeclversion(),
            opt(map(
                tuple6(
                    whitespace1(),
                    tag("encoding"),
                    whitespace0(),
                    tag("="),
                    whitespace0(),
                    delimited_string(),
                ),
                |(_, _, _, _, _, e)| e,
            )),
            opt(xmldeclstandalone()),
            whitespace0(),
            tag("?>"),
            whitespace0(),
        )((input, state)){
            Ok(((input1, mut state1), (_, _, ver, enc, sta, _, _, _))) => {
                state1.xmlversion = ver.clone();
                let res = XMLDecl {
                    version: ver,
                    encoding: enc,
                    standalone: sta,
                };
                Ok(((input1, state1), res))
            },
            Err(e) => Err(e)
        }
    }
}
