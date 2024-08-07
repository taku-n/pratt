use anyhow::*;
use async_recursion::*;

#[derive(Debug)]
enum SExpr {
    Atom(String),
    List(Vec<SExpr>),
}

impl std::fmt::Display for SExpr {  // println!("{}", x);
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SExpr::Atom(s) => write!(f, "{}", s),
            SExpr::List(l) => {
                let mut iter = l.iter();
                write!(f, "(")?;
                if let Some(head) = iter.next() {
                    write!(f, "{}", head)?;
                }
                for rest in iter {
                    write!(f, " {}", rest)?;
                }
                write!(f, ")")
            },
        }
    }
}

struct Input {
    text: String,
    position: usize,  // If you use i32, you won't use this as an index
}

impl Input {
    async fn new(text: String) -> Self {
        Self {
            text: text,
            position: 0,
        }
    }

    async fn peek(&self) -> Option<char> {  // Get a character at the current position
        self.text[self.position..].chars().next()
    }

    async fn bump(&mut self) {  // Increment the position
        self.position += self.peek().await.unwrap().len_utf8();
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let expr = String::from("-1--2");
    println!("{}", &expr);
    let mut input = Input::new(expr).await;

    let e = parse_expr(&mut input).await;
    println!("{}", &e);

    Ok(())
}

async fn parse_atom(input: &mut Input) -> SExpr {
    match input.peek().await.unwrap() {
        c if c.is_ascii_digit() => {
            input.bump().await;
            SExpr::Atom(c.into())
        },
        c => panic!("Expected an atom, got {}", c),
    }
}

#[async_recursion]
async fn parse_expr(input: &mut Input) -> SExpr {
    let leading_expr: SExpr = match input.peek().await.unwrap() {
        '-' => {
            input.bump().await;
            let following_expr = parse_expr(input).await;
            SExpr::List(vec![SExpr::Atom("-".into()), following_expr])
        },
        '(' => {
            input.bump().await;  // consumes (
            let following_expr = parse_expr(input).await;
            input.bump().await;  // consumes )
            SExpr::List(vec![SExpr::Atom("paren".into()), following_expr])
        },
        _ => parse_atom(input).await,
    };

    match input.peek().await {
        Some('?') => {
            input.bump().await;
            SExpr::List(vec![SExpr::Atom("?".into()), leading_expr])
        },
        Some('+') => {
            input.bump().await;
            let following_expr = parse_expr(input).await;
            SExpr::List(vec![SExpr::Atom("+".into()), leading_expr, following_expr])
        },
        Some('-') => {
            input.bump().await;
            let following_expr = parse_expr(input).await;
            SExpr::List(vec![SExpr::Atom("-".into()), leading_expr, following_expr])
        },
        _ => leading_expr,
    }
}
