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

#[derive(Debug, PartialEq, Eq)]
enum LeadingOpKind {
    Prefix{right_bp: i32},
    Paren,
}

#[derive(Debug, PartialEq, Eq)]
enum FollowingOpKind {
    Postfix{left_bp: i32},
    Infix{left_bp: i32, right_bp: i32},
}

impl FollowingOpKind {
    fn left_bp(&self) -> i32 {
        match self {
            FollowingOpKind::Postfix{left_bp} => *left_bp,
            FollowingOpKind::Infix{left_bp, ..} => *left_bp,
        }
    }
}

#[derive(Debug)]
struct Operator<K> {
    kind: K,
    name: String,
    symbols: Vec<char>,
}

type LeadingOp = Operator<LeadingOpKind>;
type FollowingOp = Operator<FollowingOpKind>;

#[derive(Debug)]
struct Language {
    leading_operators: Vec<LeadingOp>,
    following_operators: Vec<FollowingOp>,
}

impl Language {
    async fn new(leading_operators: Vec<LeadingOp>, following_operators: Vec<FollowingOp>) -> Self {
        Self {
            leading_operators: leading_operators,
            following_operators: following_operators,
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
    let language = Language::new(
            // -   (
            //  51  0
            vec![
                    prefix("-".into(), vec!['-'], 51).await,
                    prefix("if-then-else".into(), vec!['I', 'T', 'E'], 41).await,
                    paren("paren".into(), vec!['(', ')']).await,
            ],
            //   ?
            // 20
            //   +     -     *
            // 50 51 50 51 80 81
            vec![
                    postfix("?".into(), vec!['?'], 20).await,
                    postfix("subscript".into(), vec!['[', ']'], 100).await,
                    infix("+".into(), vec!['+'], 50, 51).await,
                    infix("-".into(), vec!['-'], 50, 51).await,
                    infix("*".into(), vec!['*'], 80, 81).await,
                    infix("=".into(), vec!['='], 21, 20).await,
            ],
    ).await;

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
    let e = parse_expr(&language, &mut input, 0).await;
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
    let e = parse_expr(&language, &mut input, 0).await;
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
    let e = parse_expr(&language, &mut input, 0).await;
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
    let e = parse_expr(&language, &mut input, 0).await;
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
    let e = parse_expr(&language, &mut input, 0).await;
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
    let e = parse_expr(&language, &mut input, 0).await;
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
    let e = parse_expr(&language, &mut input, 0).await;
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
    let e = parse_expr(&language, &mut input, 0).await;
    println!("{}", &e);

    //   1   =   2   =   I  (  3 ) T  (  4 ) E   (  5    [  6 ] )
    // 0   21 20   21 20  0  0      0  0      41  0   100 0
    //   ^
    let expr = String::from("1=2=I(3)T(4)E(5[6])");
    println!("{}", &expr);
    let mut input = Input::new(expr).await;
    let e = parse_expr(&language, &mut input, 0).await;
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

// With Binding Power
#[async_recursion]
async fn parse_expr(language: &Language, input: &mut Input, min_bp: i32) -> SExpr {
    let mut leading_expr: SExpr = async {
        let mut expr = None;
        let c = input.peek().await.unwrap();

        for leading_operator in language.leading_operators.iter() {  // Operator<LeadingOpKind>
            if leading_operator.symbols[0] == c {  // Operator<K>.symbols
                input.bump().await;
                let mut children = vec![SExpr::Atom(leading_operator.name.clone())];

                for symbol in leading_operator.symbols[1..].iter() {
                    let inner_expr = parse_expr(language, input, 0).await;
                    children.push(inner_expr);

                    // It got back because of the correct symbol
                    assert_eq!(input.peek().await.unwrap(), *symbol);
                    input.bump().await;
                }

                // If the operator is parentheses, it does not affect the expression following )
                // This is why there is LeadingOpKind::Paren not having right_bp not needed
                //
                // This block looks for the end of the effect
                // It is needed because, at the end, there is not any symbol to end
                //         unlike the just before block
                if let LeadingOpKind::Prefix{right_bp} = leading_operator.kind {
                    let following_expr = parse_expr(language, input, right_bp).await;
                    children.push(following_expr);
                }

                expr = Some(SExpr::List(children));
            }
        }

        match expr {
            Some(expr) => expr,
            None => parse_atom(input).await,  // There is not any leading expression matching
        }
    }.await;

    'main: loop {
        match input.peek().await {
            None => return leading_expr,
            Some(c) => {
                // Operator<FollowingOpKind>
                for following_operator in language.following_operators.iter() {
                    if following_operator.symbols[0] == c {
                        // If the right is not greater than the left, it ends
                        // prev-op       Atom        curr-op
                        //        min_bp      left_bp
                        if min_bp >= following_operator.kind.left_bp() {
                            return leading_expr;
                        }

                        input.bump().await;
                        let mut children
                                = vec![SExpr::Atom(following_operator.name.clone()), leading_expr];

                        for symbol in following_operator.symbols[1..].iter() {
                            let inner_expr = parse_expr(language, input, 0).await;
                            children.push(inner_expr);

                            assert_eq!(input.peek().await.unwrap(), *symbol);
                            input.bump().await;
                        }

                        // The order is different but this right_bp is still the right_bp
                        if let FollowingOpKind::Infix{right_bp, ..} = following_operator.kind {
                            let following_expr = parse_expr(language, input, right_bp).await;
                            children.push(following_expr);
                        }

                        leading_expr = SExpr::List(children);
                        continue 'main;
                    }
                }

                return leading_expr;
            },
        }
    }  // 'main: loop
}

async fn prefix(name: String, symbols: Vec<char>, right_bp: i32) -> LeadingOp {
    LeadingOp {
        kind: LeadingOpKind::Prefix{right_bp},
        name: name,
        symbols: symbols,
    }
}

async fn paren(name: String, symbols: Vec<char>) -> LeadingOp {
    LeadingOp {
        kind: LeadingOpKind::Paren,
        name: name,
        symbols: symbols,
    }
}

async fn postfix(name: String, symbols: Vec<char>, left_bp: i32) -> FollowingOp {
    FollowingOp {
        kind: FollowingOpKind::Postfix{left_bp},
        name: name,
        symbols: symbols,
    }
}

async fn infix(name: String, symbols: Vec<char>, left_bp: i32, right_bp: i32) -> FollowingOp {
    FollowingOp {
        kind: FollowingOpKind::Infix{left_bp, right_bp},
        name: name,
        symbols: symbols,
    }
}
