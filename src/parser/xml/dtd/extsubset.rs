use crate::item::Node;
use crate::parser::combinators::alt::alt10;
use crate::parser::combinators::many::many0;
use crate::parser::combinators::map::map;
use crate::parser::combinators::opt::opt;
use crate::parser::combinators::tuple::tuple2;
use crate::parser::combinators::whitespace::whitespace1;
use crate::parser::xml::dtd::attlistdecl::attlistdecl;
use crate::parser::xml::dtd::conditionals::conditionalsect;
use crate::parser::xml::dtd::elementdecl::elementdecl;
use crate::parser::xml::dtd::gedecl::gedecl;
use crate::parser::xml::dtd::notation::notation_decl;
use crate::parser::xml::dtd::pedecl::pedecl;
use crate::parser::xml::dtd::pereference::pereference;
use crate::parser::xml::dtd::textdecl::textdecl;
use crate::parser::xml::misc::{comment, processing_instruction};
use crate::parser::{ParseError, ParseInput};

pub(crate) fn extsubset<N: Node>(
) -> impl Fn(ParseInput<N>) -> Result<(ParseInput<N>, ()), ParseError> {
    move |(input, mut state)| {
        if state.standalone {
            Ok(((input, state), ()))
        } else {
            state.currentlyexternal = true;
            match tuple2(opt(textdecl()), extsubsetdecl())((input, state)) {
                Ok(((input2, mut state2), (_, _))) => {
                    if !input2.is_empty() {
                        Err(ParseError::NotWellFormed(input2.to_string()))
                    } else {
                        state2.currentlyexternal = false;
                        Ok(((input2, state2), ()))
                    }
                }
                Err(e) => Err(e),
            }
        }
    }
}

pub(crate) fn extsubsetdecl<N: Node>(
) -> impl Fn(ParseInput<N>) -> Result<(ParseInput<N>, Vec<()>), ParseError> {
    many0(alt10(
        conditionalsect(),
        elementdecl(),
        attlistdecl(),
        pedecl(),
        gedecl(),
        notation_decl(),
        whitespace1(),
        map(comment(), |_| ()),
        map(processing_instruction(), |_| ()),
        pereference(),
    ))
}
