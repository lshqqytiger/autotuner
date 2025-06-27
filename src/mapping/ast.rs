#[derive(Debug, Clone)]
pub enum Expression {
    Number(i32),
    Variable,
    Add(Box<Expression>, Box<Expression>),
    Mul(Box<Expression>, Box<Expression>),
}
