// For now, an Expression is just a VariablePath.
// We can add Literals here later if needed.
#[derive(Debug, PartialEq, Clone, Eq)]
pub enum Expression {
	Variable(Vec<String>),
}

// A full template is a sequence of literal text and expressions.
#[derive(Debug, PartialEq, Clone, Eq)]
pub enum Segment {
	Literal(String),
	Expression(Expression),
}

#[derive(Debug, PartialEq, Clone, Eq)]
pub struct AST {
	pub segments: Vec<Segment>,
}
