//! # xdm::parsexml
//!
//! A parser for XML, as a parser combinator.
//! XML 1.0, see <https://www.w3.org/TR/xml/>
//! XML 1.0 namespaces, see <http://www.w3.org/TR/xml-names/>
//! XML 1.1, see <https://www.w3.org/TR/xml11/>
//! XML 1.1 namespaces, see <http://www.w3.org/TR/xml-names11/>
//!

//extern crate nom;

use crate::parser::common::{
    is_char, is_namechar, is_pubid_char, is_pubid_charwithapos, name, ncname,
};
use crate::qname::*;
use std::collections::HashSet;
use std::str::FromStr;
use crate::xdmerror::*;

use crate::parser::combinators::alt::{alt2, alt3, alt4, alt6, alt7};
use crate::parser::combinators::delimited::delimited;
use crate::parser::combinators::expander::{genentityexpander, paramentityexpander};
use crate::parser::combinators::many::many0;
use crate::parser::combinators::many::many1;
use crate::parser::combinators::map::map;
use crate::parser::combinators::opt::opt;
use crate::parser::combinators::tag::tag;
use crate::parser::combinators::take::{take_until, take_while, take_while_m_n};
use crate::parser::combinators::tuple::tuple10;
use crate::parser::combinators::tuple::tuple2;
use crate::parser::combinators::tuple::tuple3;
use crate::parser::combinators::tuple::tuple4;
use crate::parser::combinators::tuple::tuple5;
use crate::parser::combinators::tuple::tuple6;
use crate::parser::combinators::tuple::tuple7;
use crate::parser::combinators::tuple::tuple8;
use crate::parser::combinators::tuple::tuple9;
use crate::parser::combinators::validate::validate;
use crate::parser::combinators::value::value;
use crate::parser::combinators::whitespace::{whitespace0, whitespace1};
use crate::parser::{ParseInput, ParseResult};
use crate::parser::Parserinput;

use crate::intmuttree::{
    DTDDecl, Document, DocumentBuilder, NodeBuilder, RNode, XMLDecl, XMLDeclBuilder,
};
use crate::item::{Node as ItemNode, NodeType};
use crate::value::Value;


// nom doesn't pass additional parameters, only the input,
// so this is a two-pass process.
// First, use nom to tokenize and parse the input.
// Second, use the internal structure returned by the parser
// to build the document structure.

// For backward compatibility
pub type XMLDocument = Document;

pub fn parse(e: String) -> Result<XMLDocument, Error> {
    let input = Parserinput::new(e.as_str());
    match document(input) {
        Ok((_, _, xmldoc)) => Result::Ok(xmldoc),
        Err(u) => Result::Err(Error {
            kind: ErrorKind::Unknown,
            message: format!("unrecoverable parser error at {}", u),
        }),
    }
}

fn document(input: Parserinput) -> ParseResult<XMLDocument> {
    //TODO ADD CONFIG AND DTD
    map(tuple3(opt(prolog()), element(), opt(misc())),
        |(p, e, m)| {
        let pr = p.unwrap_or((None, vec![]));

        let mut a = DocumentBuilder::new()
            .prologue(pr.1)
            .content(vec![e])
            .epilogue(m.unwrap_or(vec![]))
            .build();
        pr.0.map(|x| a.set_xmldecl(x));
        a
    })((input,0))
}

// prolog ::= XMLDecl misc* (doctypedecl Misc*)?
fn prolog() -> impl Fn(ParseInput) -> ParseResult<(Option<XMLDecl>, Vec<RNode>)> {
    map(
        tuple4(opt(xmldecl()), misc(), opt(doctypedecl()), misc()),
        |(xmld, mut m1, _dtd, mut m2)| {
            m1.append(&mut m2);
            (xmld, m1)
        },
    )
}

fn xmldecl() -> impl Fn(ParseInput) -> ParseResult<XMLDecl> {
    map(
        tuple10(
            tag("<?xml"),
            whitespace1(),
            map(
                tuple5(
                    tag("version"),
                    whitespace0(),
                    tag("="),
                    whitespace0(),
                    delimited_string(),
                ),
                |(_, _, _, _, v)| v,
            ),
            whitespace1(),
            opt(map(
                tuple5(
                    tag("encoding"),
                    whitespace0(),
                    tag("="),
                    whitespace0(),
                    delimited_string(),
                ),
                |(_, _, _, _, e)| e,
            )),
            whitespace1(),
            opt(map(
                tuple5(
                    tag("standalone"),
                    whitespace0(),
                    tag("="),
                    whitespace0(),
                    delimited_string(),
                ),
                |(_, _, _, _, s)| s,
            )),
            whitespace0(),
            tag("?>"),
            whitespace0(),
        ),
        |(_, _, ver, _, enc, _, sta, _, _, _)| XMLDecl {
            version: ver,
            encoding: enc,
            standalone: sta,
        },
    )
}

fn doctypedecl() -> impl Fn(ParseInput) -> ParseResult<()> {
    move |input| match tuple8(
        tag("<!DOCTYPE"),
        whitespace1(),
        name(),
        whitespace1(),
        opt(externalid()),
        whitespace0(),
        opt(delimited(tag("["), intsubset(), tag("]"))),
        tag(">"),
    )(input)
    {
        Ok((d, i, (_, _, _n, _, _e, _, _inss, _))) => Ok((d, i, ())),
        Err(err) => Err(err),
    }
}

fn externalid() -> impl Fn(ParseInput) -> ParseResult<(String, Option<String>)> {
    alt2(
        map(
            tuple3(
                tag("SYSTEM"),
                whitespace0(),
                alt2(
                    delimited(tag("'"), take_until("'"), tag("'")),
                    delimited(tag("\""), take_until("\""), tag("\"")),
                ), //SystemLiteral
            ),
            |(_, _, sid)| (sid, None),
        ),
        map(
            tuple5(
                tag("PUBLIC"),
                whitespace0(),
                alt2(
                    delimited(tag("'"), take_while(|c| !is_pubid_char(&c)), tag("'")),
                    delimited(
                        tag("\""),
                        take_while(|c| !is_pubid_charwithapos(&c)),
                        tag("\""),
                    ),
                ), //PubidLiteral TODO validate chars here (PubidChar from spec).
                whitespace1(),
                alt2(
                    delimited(tag("'"), take_until("'"), tag("'")),
                    delimited(tag("\""), take_until("\""), tag("\"")),
                ), //SystemLiteral
            ),
            |(_, _, pid, _, sid)| (sid, Some(pid)),
        ),
    )
}

fn intsubset() -> impl Fn(ParseInput) -> ParseResult<Vec<()>> {
    many0(alt6(
        elementdecl(),
        attlistdecl(),
        pedecl(),
        gedecl(),
        ndatadecl(),
        whitespace1(),
    ))
}

//elementdecl	   ::=   	'<!ELEMENT' S Name S contentspec S? '>'
fn elementdecl() -> impl Fn(ParseInput) -> ParseResult<()> {
    move |input| match tuple7(
        tag("<!ELEMENT"),
        whitespace1(),
        qualname(),
        whitespace1(),
        contentspec(), //contentspec - TODO Build out.
        whitespace0(),
        tag(">"),
    )(input)
    {
        Ok((mut d, i, (_, _, n, _, s, _, _))) => {
            d.dtd.elements.insert(n.to_string(), DTDDecl::Element(n, s));
            Ok((d, i, ()))
        }
        Err(err) => Err(err),
    }
}
fn contentspec() -> impl Fn(ParseInput) -> ParseResult<String> {
    alt4(
        value(tag("EMPTY"), "EMPTY".to_string()),
        value(tag("ANY"), "ANY".to_string()),
        mixed(),
        children(),
    )
}

//AttlistDecl ::= '<!ATTLIST' S Name AttDef* S? '>'
fn attlistdecl() -> impl Fn(ParseInput) -> ParseResult<()> {
    move |input| match tuple6(
        tag("<!ATTLIST"),
        whitespace1(),
        qualname(),
        many0(attdef()),
        whitespace0(),
        tag(">"),
    )(input)
    {
        Ok((mut d, i, (_, _, n, _, _, _))) => {
            d.dtd
                .attlists
                .insert(n.to_string(), DTDDecl::Attlist(n, "".to_string()));
            Ok((d, i, ()))
        }
        Err(err) => Err(err),
    }
}

//AttDef ::= S Name S AttType S DefaultDecl
fn attdef() -> impl Fn(ParseInput) -> ParseResult<String> {
    map(
        tuple6(
            whitespace1(),
            name(),
            whitespace1(),
            atttype(),
            whitespace1(),
            defaultdecl(),
        ),
        |_x| "".to_string(),
    )
}

//AttType ::= StringType | TokenizedType | EnumeratedType
fn atttype() -> impl Fn(ParseInput) -> ParseResult<()> {
    alt3(
        tag("CDATA"), //Stringtype
        alt7(
            //tokenizedtype
            tag("ID"),
            tag("IDREF"),
            tag("IDREFS"),
            tag("ENTITY"),
            tag("ENTITIES"),
            tag("NMTOKEN"),
            tag("NMTOKENS"),
        ),
        enumeratedtype(),
    )
}

//EnumeratedType ::= NotationType | Enumeration
fn enumeratedtype() -> impl Fn(ParseInput) -> ParseResult<()> {
    alt2(notationtype(), enumeration())
}

//NotationType ::= 'NOTATION' S '(' S? Name (S? '|' S? Name)* S? ')'
fn notationtype() -> impl Fn(ParseInput) -> ParseResult<()> {
    map(
        tuple8(
            tag("NOTATION"),
            whitespace1(),
            tag("("),
            whitespace0(),
            name(),
            many0(tuple4(whitespace0(), tag("|"), whitespace0(), name())),
            whitespace0(),
            tag(")"),
        ),
        |_x| (),
    )
}

//Enumeration ::= '(' S? Nmtoken (S? '|' S? Nmtoken)* S? ')'
fn enumeration() -> impl Fn(ParseInput) -> ParseResult<()> {
    map(
        tuple6(
            tag("("),
            whitespace0(),
            nmtoken(),
            many0(tuple4(whitespace0(), tag("|"), whitespace0(), nmtoken())),
            whitespace0(),
            tag(")"),
        ),
        |_x| (),
    )
}

fn nmtoken() -> impl Fn(ParseInput) -> ParseResult<()> {
    map(many1(take_while(|c| is_namechar(&c))), |_x| ())
}

//DefaultDecl ::= '#REQUIRED' | '#IMPLIED' | (('#FIXED' S)? AttValue)
fn defaultdecl() -> impl Fn(ParseInput) -> ParseResult<()> {
    map(
        alt3(
            value(tag("#REQUIRED"), "#REQUIRED".to_string()),
            value(tag("#IMPLIED"), "#IMPLIED".to_string()),
            map(
                tuple2(
                    opt(tuple2(
                        value(tag("#FIXED"), "#FIXED".to_string()),
                        whitespace1(),
                    )),
                    attvalue(),
                ),
                |(x, y)| match x {
                    None => y,
                    Some((mut f, _)) => {
                        f.push_str(&y);
                        f
                    }
                },
            ),
        ),
        |_x| (),
    )
}

//AttValue ::= '"' ([^<&"] | Reference)* '"' | "'" ([^<&'] | Reference)* "'"
fn attvalue() -> impl Fn(ParseInput) -> ParseResult<String> {
    alt2(
        delimited(
            tag("\'"),
            map(
                many0(alt3(
                    take_while(|c| !"&\'<".contains(c)),
                    genentityexpander(),
                    paramentityexpander(),
                )),
                |v| v.join(""),
            ),
            tag("\'"),
        ),
        delimited(
            tag("\""),
            map(
                many0(alt3(
                    take_while(|c| !"&\"<".contains(c)),
                    genentityexpander(),
                    paramentityexpander(),
                )),
                |v| v.join(""),
            ),
            tag("\""),
        ),
    )
}

fn pedecl() -> impl Fn(ParseInput) -> ParseResult<()> {
    move |input| match tuple9(
        tag("<!ENTITY"),
        whitespace1(),
        tag("%"),
        whitespace1(),
        qualname(),
        whitespace1(),
        alt2(
            delimited(tag("'"), take_until("'"), tag("'")),
            delimited(tag("\""), take_until("\""), tag("\"")),
        ),
        whitespace0(),
        tag(">"),
    )(input)
    {
        Ok((mut d, i, (_, _, _, _, n, _, s, _, _))) => {
            d.dtd
                .paramentities
                .insert(n.to_string(), DTDDecl::ParamEntity(n, s));
            Ok((d, i, ()))
        }
        Err(err) => Err(err),
    }
}

fn gedecl() -> impl Fn(ParseInput) -> ParseResult<()> {
    move |input| match tuple7(
        tag("<!ENTITY"),
        whitespace1(),
        qualname(),
        whitespace1(),
        alt2(
            delimited(tag("'"), take_until("'"), tag("'")),
            delimited(tag("\""), take_until("\""), tag("\"")),
        ),
        whitespace0(),
        tag(">"),
    )(input)
    {
        Ok((mut d, i, (_, _, n, _, s, _, _))) => {
            d.dtd
                .generalentities
                .insert(n.to_string(), DTDDecl::GeneralEntity(n, s));
            Ok((d, i, ()))
        }
        Err(err) => Err(err),
    }
}
fn ndatadecl() -> impl Fn(ParseInput) -> ParseResult<()> {
    move |input| match tuple7(
        tag("<!NOTATION"),
        whitespace1(),
        qualname(),
        whitespace1(),
        take_until(">"), //contentspec - TODO Build out.
        whitespace0(),
        tag(">"),
    )(input)
    {
        Ok((mut d, i, (_, _, n, _, s, _, _))) => {
            d.dtd
                .notations
                .insert(n.to_string(), DTDDecl::Notation(n, s));
            Ok((d, i, ()))
        }
        Err(err) => Err(err),
    }
}

//Mixed	   ::=   	'(' S? '#PCDATA' (S? '|' S? Name)* S? ')*' | '(' S? '#PCDATA' S? ')'
fn mixed() -> impl Fn(ParseInput) -> ParseResult<String> {
    alt2(
        map(
            tuple6(
                tag("("),
                whitespace0(),
                tag("#PCDATA"),
                many0(tuple4(whitespace0(), tag("|"), whitespace0(), name())),
                whitespace0(),
                tag(")*"),
            ),
            |_x| "".to_string(),
        ),
        map(
            tuple5(
                tag("("),
                whitespace0(),
                tag("#PCDATA"),
                whitespace0(),
                tag(")"),
            ),
            |_x| "".to_string(),
        ),
    )
}

// children	   ::=   	(choice | seq) ('?' | '*' | '+')?
fn children() -> impl Fn(ParseInput) -> ParseResult<String> {
    map(
        tuple2(
            alt2(choice(), seq()),
            opt(alt3(tag("?"), tag("*"), tag("+"))),
        ),
        |_x| "".to_string(),
    )
}

// cp	   ::=   	(Name | choice | seq) ('?' | '*' | '+')?
fn cp() -> impl Fn(ParseInput) -> ParseResult<String> {
    move |input| {
        map(
            tuple2(
                alt3(name(), choice(), seq()),
                opt(alt3(tag("?"), tag("*"), tag("+"))),
            ),
            |_x| "".to_string(),
        )(input)
    }
}
//choice	   ::=   	'(' S? cp ( S? '|' S? cp )+ S? ')'
fn choice() -> impl Fn(ParseInput) -> ParseResult<String> {
    move |input| {
        map(
            tuple6(
                tag("("),
                whitespace0(),
                cp(),
                many0(tuple4(whitespace0(), tag("|"), whitespace0(), cp())),
                whitespace0(),
                tag(")"),
            ),
            |_x| "".to_string(),
        )(input)
    }
}

//seq	   ::=   	'(' S? cp ( S? ',' S? cp )* S? ')'
fn seq() -> impl Fn(ParseInput) -> ParseResult<String> {
    map(
        tuple6(
            tag("("),
            whitespace0(),
            cp(),
            many0(tuple4(whitespace0(), tag(","), whitespace0(), cp())),
            whitespace0(),
            tag(")"),
        ),
        |_x| "".to_string(),
    )
}

// Element ::= EmptyElemTag | STag content ETag
fn element() -> impl Fn(ParseInput) -> ParseResult<RNode> {
    move |input|
        //map(
        alt2(
            emptyelem(),
            taggedelem(),
        )
            //,|e| {
            // TODO: Check for namespace declarations, and resolve URIs in the node tree under 'e'
//            e
//        }
            //)
            (input)
}

// EmptyElemTag ::= '<' Name (Attribute)* '/>'
fn emptyelem() -> impl Fn(ParseInput) -> ParseResult<RNode> {
    map(
        tuple5(
            tag("<"),
            qualname(),
            attributes(), //many0(attribute),
            whitespace0(),
            tag("/>"),
        ),
        |(_, n, av, _, _)| {
            let e = NodeBuilder::new(NodeType::Element).name(n).build();
            av.iter()
                .for_each(|b| e.add_attribute(b.clone()).expect("unable to add attribute"));
            e
        },
    )
}

// STag ::= '<' Name (Attribute)* '>'
// ETag ::= '</' Name '>'
// NB. Names must match
fn taggedelem() -> impl Fn(ParseInput) -> ParseResult<RNode> {
    map(
        validate(
            tuple10(
                tag("<"),
                qualname(),
                attributes(), //many0(attribute),
                whitespace0(),
                tag(">"),
                content(),
                tag("</"),
                qualname(),
                whitespace0(),
                tag(">"),
            ),
            |(_, n, _a, _, _, _c, _, e, _, _)| n.to_string() == e.to_string(),
        ),
    |(_, n, av, _, _, c, _, _e, _, _)| {
            // TODO: check that the start tag name and end tag name match (n == e)
            let mut a = NodeBuilder::new(NodeType::Element).name(n).build();
            av.iter()
                .for_each(|b| a.add_attribute(b.clone()).expect("unable to add attribute"));
            c.iter().for_each(|d| {
                a.push(d.clone()).expect("unable to add node");
            });
            a
        },
    )
}

// QualifiedName

fn qualname() -> impl Fn(ParseInput) -> ParseResult<QualifiedName> {
    alt2(prefixed_name(), unprefixed_name())
}
fn unprefixed_name() -> impl Fn(ParseInput) -> ParseResult<QualifiedName> {
    map(ncname(), |localpart| {
        QualifiedName::new(None, None, localpart)
    })
}
fn prefixed_name() -> impl Fn(ParseInput) -> ParseResult<QualifiedName> {
    map(
        tuple3(ncname(), tag(":"), ncname()),
        |(prefix, _, localpart)| QualifiedName::new(None, Some(prefix), localpart),
    )
}

fn attributes() -> impl Fn(ParseInput) -> ParseResult<Vec<RNode>> {
    //this is just a wrapper around the attribute function, that checks for duplicates.
    validate(many0(attribute()), |v: &Vec<RNode>| {
        let attrs = v.clone();
        //Check if the xml:space attribute is present and if so, does it have
        //"Preserved" or "Default" as its value
        for a in attrs.clone() {
            if a.name().get_prefix() == Some("xml".to_string()) &&
                a.name().get_localname() == *"space" &&
                !(a.to_string() == "Default" || a.to_string() == "Preserve") {
                    return false
                }
            /*
            match a.name(){
                QualifiedName {nsuri, prefix, localname } => {
                    if prefix == Some("xml".to_string()) && localname == "space".to_string() {
                        if !(a.to_string() == "Default" || a.to_string() == "Preserve") {
                            return false
                        }
                    }
                }
            }
             */
        }

        //Check if duplicates
        let uniqueattrs: HashSet<_> = attrs
            .iter()
            .map(|xmlnode| match xmlnode.node_type() {
                NodeType::Attribute => xmlnode.name().to_string(),
                _ => "".to_string(),
            })
            .collect();
        v.len() == uniqueattrs.len()
    })
}
// Attribute ::= Name '=' AttValue
fn attribute() -> impl Fn(ParseInput) -> ParseResult<RNode> {
    map(
        tuple6(
            whitespace1(),
            qualname(),
            whitespace0(),
            tag("="),
            whitespace0(),
            delimited_string(),
        ),
        |(_, n, _, _, _, s)| {
            NodeBuilder::new(NodeType::Attribute)
                .name(n)
                .value(Value::String(s))
                .build()
        },
    )
}
fn delimited_string() -> impl Fn(ParseInput) -> ParseResult<String> {
    alt2(string_single(), string_double())
}
fn string_single() -> impl Fn(ParseInput) -> ParseResult<String> {
    delimited(
        tag("\'"),
        map(
            many0(alt3(
                chardata_escapes(),
                chardata_unicode_codepoint(),
                take_while(|c| !"&\'<".contains(c)),
            )),
            |v| v.concat(),
        ),
        tag("\'"),
    )
}
fn string_double() -> impl Fn(ParseInput) -> ParseResult<String> {
    delimited(
        tag("\""),
        map(
            many0(alt2(
                chardata_escapes(),
                take_while(|c| !"&\"<".contains(c)),
            )),
            |v| v.concat(),
        ),
        tag("\""),
    )
}

// content ::= CharData? ((element | Reference | CDSect | PI | Comment) CharData?)*
fn content() -> impl Fn(ParseInput) -> ParseResult<Vec<RNode>> {
    map(
        tuple2(
            opt(chardata()),
            many0(tuple2(
                alt4(
                    element(),
                    reference(),
                    // TODO: CData Section
                    processing_instruction(),
                    comment(),
                ),
                opt(chardata()),
            )),
        ),
        |(c, v)| {
            let mut new: Vec<RNode> = Vec::new();
            if c.is_some() {
                new.push(
                    NodeBuilder::new(NodeType::Text)
                        .value(Value::String(c.unwrap()))
                        .build(),
                );
            }
            if !v.is_empty() {
                for (w, d) in v {
                    new.push(w);
                    if d.is_some() {
                        new.push(
                            NodeBuilder::new(NodeType::Text)
                                .value(Value::String(d.unwrap()))
                                .build(),
                        );
                    }
                }
            }
            new
        },
    )
}

// Reference ::= EntityRef | CharRef
// TODO
fn reference() -> impl Fn(ParseInput) -> ParseResult<RNode> {
    map(genentityexpander(), |_| {
        NodeBuilder::new(NodeType::Text)
            .value(Value::from(""))
            .build()
    })
}

// PI ::= '<?' PITarget (char* - '?>') '?>'
fn processing_instruction() -> impl Fn(ParseInput) -> ParseResult<RNode> {
    validate(
        map(
            tuple5(
                tag("<?"),
                name(),
                opt(tuple2(whitespace1(), take_until("?>"))),
                whitespace0(),
                tag("?>"),
            ),
            |(_, n, vt, _, _)| match vt {
                None => {
                    NodeBuilder::new(NodeType::ProcessingInstruction)
                        .pi_name(n)
                        .value(Value::String("".to_string()))
                        .build()
                },
                Some((_, v)) => {
                    NodeBuilder::new(NodeType::ProcessingInstruction)
                        .pi_name(n)
                        .value(Value::String(v))
                        .build()
                }
            },
        ),
        |v| match v.node_type() {
            NodeType::ProcessingInstruction => {
                if v.to_string().contains(|c: char| !is_char(&c)){
                    false
                } else {
                    v.name().get_localname() != *"xml"
                    /*
                    match v.name(){
                        QualifiedName {nsuri, prefix, localname} => {
                            localname.to_lowercase() != "xml"
                        }
                    }
                     */
                }
            },
            _ => false
        },
    )
}

// Comment ::= '<!--' (char* - '--') '-->'
fn comment() -> impl Fn(ParseInput) -> ParseResult<RNode> {
    validate(
        map(
            delimited(tag("<!--"), take_until("--"), tag("-->")),
            |v: String| {
                NodeBuilder::new(NodeType::Comment)
                    .value(Value::String(v))
                    .build()
            }),
        |v| match v.node_type() {
            NodeType::Comment => {!v.to_string().contains(|c: char| !is_char(&c))}
            _ => false
        },
    )
}

// Misc ::= Comment | PI | S
fn misc() -> impl Fn(ParseInput) -> ParseResult<Vec<RNode>> {
    map(
        tuple2(
            many0(map(
                alt2(
                    tuple2(whitespace0(), comment()),
                    tuple2(whitespace0(), processing_instruction()),
                ),
                |(_ws, xn)| xn,
            )),
            whitespace0(),
        ),
        |(v, _)| v,
    )
}

// CharData ::= [^<&]* - (']]>')
fn chardata() -> impl Fn(ParseInput) -> ParseResult<String> {
    map(
        many1(alt3(
            chardata_cdata(),
            chardata_escapes(),
            chardata_literal(),
        )),
        |v| v.concat(),
    )
}

fn chardata_cdata() -> impl Fn(ParseInput) -> ParseResult<String> {
    delimited(tag("<![CDATA["), take_until("]]>"), tag("]]>"))
}

fn chardata_escapes() -> impl Fn(ParseInput) -> ParseResult<String> {
    move |(input, index)| match chardata_unicode_codepoint()((input.clone(), index)) {
        Ok((inp, ind, s)) => Ok((inp, ind, s)),
        Err(e) => match delimited(tag("&"), take_until(";"), tag(";"))((input, index)) {
            Ok((inp, ind, rstr)) => match rstr.as_str() {
                "gt" => Ok((inp, ind, ">".to_string())),
                "lt" => Ok((inp, ind, "<".to_string())),
                "amp" => Ok((inp, ind, "&".to_string())),
                "quot" => Ok((inp, ind, "\"".to_string())),
                "apos" => Ok((inp, ind, "\'".to_string())),
                _ => Err(e),
            },
            Err(e) => Err(e),
        },
    }
}

fn chardata_unicode_codepoint() -> impl Fn(ParseInput) -> ParseResult<String> {
    map(
        alt2(
            delimited(tag("&#x"), parse_hex(), tag(";")),
            delimited(tag("&#"), parse_decimal(), tag(";")),
        ),
        |value| std::char::from_u32(value).unwrap().to_string(),
    )
}
fn parse_hex() -> impl Fn(ParseInput) -> ParseResult<u32> {
    map(
        take_while_m_n(1, 6, |c: char| c.is_ascii_hexdigit()),
        |hex| u32::from_str_radix(&hex, 16).unwrap(),
    )
}
fn parse_decimal() -> impl Fn(ParseInput) -> ParseResult<u32> {
    map(take_while_m_n(1, 6, |c: char| c.is_ascii_digit()), |dec| {
        u32::from_str(&dec).unwrap()
    })
}

fn chardata_literal() -> impl Fn(ParseInput) -> ParseResult<String> {
    validate(take_while(|c| c != '<' && c != '&'), |s| !s.contains("]]>"))
}
