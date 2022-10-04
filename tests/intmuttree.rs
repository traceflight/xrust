use xrust::evaluate_tests;
use xrust::intmuttree::{Document, NodeBuilder, RNode};
use xrust::item::{Node, NodeType};
use xrust::item_node_tests;
use xrust::item_value_tests;
use xrust::qname::QualifiedName;
use xrust::xpath_tests;

// Run the generic Item/Value tests
item_value_tests!(RNode);

fn make_empty_doc() -> RNode {
    NodeBuilder::new(NodeType::Document).build()
}

fn make_doc(n: QualifiedName, v: Value) -> RNode {
    let mut d = NodeBuilder::new(NodeType::Document).build();
    let mut child = NodeBuilder::new(NodeType::Element).name(n).build();
    d.push(child.clone()).expect("unable to append child");
    child
        .push(NodeBuilder::new(NodeType::Text).value(v).build())
        .expect("unable to append child");
    d
}

fn make_sd() -> Rc<Item<RNode>> {
    let e = Document::try_from(
        "<a><b><a><b/><b/></a><a><b/><b/></a></b><b><a><b/><b/></a><a><b/><b/></a></b></a>",
    )
    .expect("failed to parse XML")
    .content[0]
        .clone();
    let mut d = NodeBuilder::new(NodeType::Document).build();
    d.push(e).expect("unable to append node");
    Rc::new(Item::Node(d))
}

item_node_tests!(make_empty_doc, make_doc);
evaluate_tests!(make_empty_doc);
xpath_tests!(make_empty_doc, make_sd);
