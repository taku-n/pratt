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
    // -   1   -   -   2
    //  51   50 51  51
    // (- 1)   -   -   2
    //       50 51  51
    // (- 1)   -   (- 2)
    //       50 51
    // (- (- 1) (- 2))
    let expr = String::from("-1--2");
    println!("{}", &expr);
    let mut input = Input::new(expr).await;
    let e = parse_expr(&mut input, 0).await;
    println!("{}", &e);

    //   1   +   2   *   3
    // 0   50 51   80 81
    //   ^
    // 1   +   (* 2 3)
    //   50 51
    //                 ^
    // (+ 1 (* 2 3))
    //               ^
    let expr = String::from("1+2*3");
    println!("{}", &expr);
    let mut input = Input::new(expr).await;
    let e = parse_expr(&mut input, 0).await;
    println!("{}", &e);

    //   1   *   2   +   3
    // 0   80 81   50 51
    //   ^
    //   (* 1 2)   +   3
    // 0         50 51
    //             ^
    //             leading: (* 1 2)
    // (+ (* 1 2) 3)
    //               ^
    let expr = String::from("1*2+3");
    println!("{}", &expr);
    let mut input = Input::new(expr).await;
    let e = parse_expr(&mut input, 0).await;
    println!("{}", &e);

    //   1   *   (  2   +   3 )
    // 0   80 81  0   50 51
    //   ^
    //   1   *   (paren (+ 2 3))
    // 0   80 81
    //                           ^
    //                           leading: 1
    // (* 1 (paren (+ 2 3)))
    //                       ^
    let expr = String::from("1*(2+3)");
    println!("{}", &expr);
    let mut input = Input::new(expr).await;
    let e = parse_expr(&mut input, 0).await;
    println!("{}", &e);

    //   -   1   +   2
    // 0  51   50 51
    //   ^
    //   (- 1)   +   2
    // 0       50 51
    //           ^
    //           leading: (- 1)
    // (+ (- 1) 2)
    //             ^
    let expr = String::from("-1+2");
    println!("{}", &expr);
    let mut input = Input::new(expr).await;
    let e = parse_expr(&mut input, 0).await;
    println!("{}", &e);

    //   -   1   *   2
    // 0  51   80 81
    //   ^
    // (- (* 1 2))
    //             ^
    // When the following binding power is always greater,
    // the position goes to the end without stops
    let expr = String::from("-1*2");
    println!("{}", &expr);
    let mut input = Input::new(expr).await;
    let e = parse_expr(&mut input, 0).await;
    println!("{}", &e);

    //   1   *   2   ?
    // 0   80 81   20
    //   ^
    //   (* 1 2)   ?
    // 0         20
    //             ^
    //             leading: (* 1 2)
    // (? (* 1 2))
    //             ^
    let expr = String::from("1*2?");
    println!("{}", &expr);
    let mut input = Input::new(expr).await;
    let e = parse_expr(&mut input, 0).await;
    println!("{}", &e);

    //   -   1   *   2   ?
    // 0  51   80 81   20
    //   ^
    //   -   (* 1 2)   ?
    // 0  51         20
    //                 ^
    //                 leading: (* 1 2)
    //   (- (* 1 2))   ?
    // 0             20
    //                 ^
    //                 leading: (- (* 1 2))
    // (? (- (* 1 2)))
    //                 ^
    let expr = String::from("-1*2?");
    println!("{}", &expr);
    let mut input = Input::new(expr).await;
    let e = parse_expr(&mut input, 0).await;
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
async fn parse_expr(input: &mut Input, min_bp: i32) -> SExpr {  // with Binding Power
    // -   (
    //  51  0
    let mut leading_expr: SExpr = match input.peek().await.unwrap() {
        '-' => {
            const NEG_RBP: i32 = 51;
            input.bump().await;
            let following_expr = parse_expr(input, NEG_RBP).await;
            SExpr::List(vec![SExpr::Atom("-".into()), following_expr])
        },
        '(' => {
            const PAREN_RBP: i32 = 0;
            input.bump().await;  // consumes (
            let following_expr = parse_expr(input, PAREN_RBP).await;
            input.bump().await;  // consumes )
            SExpr::List(vec![SExpr::Atom("paren".into()), following_expr])
        },
        _ => parse_atom(input).await,
    };

    //   ?
    // 20
    //   +     -     *
    // 50 51 50 51 80 81
    loop {
        match input.peek().await {
            None => return leading_expr,
            Some(c) => {
                // If the right is not greater than the left, it ends
                // prev-op       Atom   curr-op
                //        min_bp      bp
                // Does the expression match the pattern?
                if matches!(following_operator_lbp(c).await, Some(bp) if min_bp >= bp) {
                    return leading_expr;
                }
            },
        }

        match input.peek().await {
            Some('?') => {
                input.bump().await;
                leading_expr = SExpr::List(vec![SExpr::Atom("?".into()), leading_expr]);
            },
            Some('+') => {
                const PLUS_RBP: i32 = 51;
                input.bump().await;
                let following_expr = parse_expr(input, PLUS_RBP).await;
                leading_expr
                        = SExpr::List(vec![SExpr::Atom("+".into()), leading_expr, following_expr]);
            },
            Some('-') => {
                const MINUS_RBP: i32 = 51;
                input.bump().await;
                let following_expr = parse_expr(input, MINUS_RBP).await;
                leading_expr
                        = SExpr::List(vec![SExpr::Atom("-".into()), leading_expr, following_expr]);
            },
            Some('*') => {
                const MULT_RBP: i32 = 81;
                input.bump().await;
                let following_expr = parse_expr(input, MULT_RBP).await;
                leading_expr
                        = SExpr::List(vec![SExpr::Atom("*".into()), leading_expr, following_expr]);
            },
            _ => return leading_expr,
        }
    }
}

async fn following_operator_lbp(operator: char) -> Option<i32> {
    match operator {
        '?' => Some(20),
        '+' => Some(50),
        '-' => Some(50),
        '*' => Some(80),
        _ => None,
    }
}
