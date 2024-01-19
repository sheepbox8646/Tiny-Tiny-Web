/* Tiny Tiny Web
 * Copyright (C) 2024 Plasma (https://github.com/duoduo70/Tiny-Tiny-Web/).
 *
 * You should have received a copy of the GNU General Public License
 * along with this program;
 * if not, see <https://www.gnu.org/licenses/>.
 */
use std::{collections::HashMap, fmt::Display, io::Write, rc::Rc};

//TODO: Standardize error reporting
//TODO: i18n support

#[derive(Clone)]
enum Expression {
    Symbol(String),
    Number(f64),
    List(Vec<Expression>),
    Func(fn(&[Expression]) -> Result<Expression, GError>),
    Bool(bool),
    Lambda(Lambda),
    String(String),
}

#[derive(Clone)]
struct Lambda {
    params: Rc<Expression>,
    body: Rc<Expression>,
}

impl Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expression::Symbol(a) => write!(f, "{}", a),
            Expression::Number(a) => write!(f, "{}", a),
            Expression::List(a) => write!(
                f,
                "{:?}",
                a.iter().map(|e| { format!("{}", e) }).collect::<Vec<_>>()
            ),
            Expression::Bool(a) => write!(f, "{}", a),
            Expression::Lambda(a) => {
                write!(f, "lambda: {{ params: {} , body: {} }}", &a.params, &a.body)
            }
            Expression::String(a) => write!(f, "\"{}\"", a),
            Expression::Func(_) => write!(f, "function()"), // TODO: Display function sign
        }
    }
}

enum GError {
    Reason(String),
}

struct Environment<'a> {
    data: HashMap<String, Expression>,
    outer: Option<&'a Environment<'a>>,
}

fn eval_lambda_args(args: &[Expression]) -> Result<Expression, GError> {
    let params = args
        .first()
        .ok_or(GError::Reason(format!("unexpected args form")))?;
    let body = args
        .get(1)
        .ok_or(GError::Reason(format!("unexpected second form")))?;
    if args.len() != 2 {
        return Err(GError::Reason(format!("lambda can only have two forms")));
    }
    Ok(Expression::Lambda(Lambda {
        params: Rc::new(params.clone()),
        body: Rc::new(body.clone()),
    }))
}

fn tokenize(expr: String) -> Vec<String> {
    expr.replace("(", " ( ")
        .replace(")", " ) ")
        .split_whitespace()
        .map(|x| x.to_string())
        .collect()
}

fn parse<'a>(tokens: &'a [String]) -> Result<(Expression, &'a [String]), GError> {
    let (token, rest) = tokens
        .split_first()
        .ok_or(GError::Reason("could not get token".to_string()))?;
    match &token[..] {
        "(" => read_seq(rest),
        ")" => Err(GError::Reason("unexpected `)`".to_string())),
        _ => Ok((parse_atom(token), rest)),
    }
}

fn read_seq<'a>(tokens: &'a [String]) -> Result<(Expression, &'a [String]), GError> {
    let mut res: Vec<Expression> = vec![];
    let mut xs = tokens;
    loop {
        let (next_token, rest) = xs
            .split_first()
            .ok_or(GError::Reason("could not find closing `)`".to_string()))?;
        if next_token == ")" {
            return Ok((Expression::List(res), rest));
        }
        let (exp, new_xs) = parse(&xs)?;
        res.push(exp);
        xs = new_xs;
    }
}

fn parse_atom(token: &str) -> Expression {
    if token.len() >= 2
        && token.bytes().nth(0).unwrap() == b'\"'
        && token.bytes().nth(token.len() - 1).unwrap() == b'\"'
    {
        return Expression::String(token[1..token.len() - 1].to_string());
    }
    match token {
        "true" => Expression::Bool(true),
        "false" => Expression::Bool(false),
        _ => {
            let potential_float = token.parse();
            match potential_float {
                Ok(v) => Expression::Number(v),
                Err(_) => Expression::Symbol(token.to_string().clone()),
            }
        }
    }
}

fn default_env<'a>() -> Environment<'a> {
    let mut data: HashMap<String, Expression> = HashMap::new();
    data.insert(
        "+".to_string(),
        Expression::Func(|args: &[Expression]| -> Result<Expression, GError> {
            let sum = parse_list_of_floats(args)?
                .iter()
                .fold(0.0, |sum, a| sum + a);
            Ok(Expression::Number(sum))
        }),
    );
    data.insert(
        "-".to_string(),
        Expression::Func(|args: &[Expression]| -> Result<Expression, GError> {
            let floats = parse_list_of_floats(args)?;
            let first = *floats
                .first()
                .ok_or(GError::Reason("expected at least one number".to_string()))?;
            let sum_of_rest = floats[1..].iter().fold(0.0, |sum, a| sum + a);

            Ok(Expression::Number(first - sum_of_rest))
        }),
    );

    data.insert(
        "=".to_string(),
        Expression::Func(|args: &[Expression]| -> Result<Expression, GError> {
            let floats = parse_list_of_floats(args)?;
            // 要想比较，需要有两个值
            if floats.len() != 2 {
                return Err(GError::Reason("expected two number".to_string()));
            }
            // 将第 0 个元素和第 1 个元素进行比较
            if floats.get(0).is_none() || floats.get(1).is_none() {
                return Err(GError::Reason("expected number".to_string()));
            }
            let is_ok = floats.get(0).unwrap().eq(floats.get(1).unwrap());
            Ok(Expression::Bool(is_ok))
        }),
    );

    macro_rules! ensure_tonicity {
        ($check_fn:expr) => {{
            |args: &[Expression]| -> Result<Expression, GError> {
                let floats = parse_list_of_floats(args)?;
                let first = floats
                    .first()
                    .ok_or(GError::Reason("expected at least one number".to_string()))?;
                let rest = &floats[1..];
                fn f(prev: &f64, xs: &[f64]) -> bool {
                    match xs.first() {
                        Some(x) => $check_fn(prev, x) && f(x, &xs[1..]),
                        None => true,
                    }
                }
                Ok(Expression::Bool(f(first, rest)))
            }
        }};
    }

    data.insert(
        ">".to_string(),
        Expression::Func(ensure_tonicity!(|a, b| a > b)),
    );

    data.insert(
        "<".to_string(),
        Expression::Func(ensure_tonicity!(|a, b| a < b)),
    );

    data.insert(
        "<=".to_string(),
        Expression::Func(ensure_tonicity!(|a, b| a <= b)),
    );

    data.insert(
        ">=".to_string(),
        Expression::Func(ensure_tonicity!(|a, b| a >= b)),
    );

    Environment { data, outer: None }
}

fn parse_list_of_floats(args: &[Expression]) -> Result<Vec<f64>, GError> {
    args.iter().map(|x| parse_single_float(x)).collect()
}

fn parse_single_float(exp: &Expression) -> Result<f64, GError> {
    match exp {
        Expression::Number(num) => Ok(*num),
        _ => Err(GError::Reason("expect a number".to_string())),
    }
}

fn env_for_lambda<'a>(
    params: Rc<Expression>,
    args: &[Expression],
    outer_env: &'a mut Environment,
) -> Result<Environment<'a>, GError> {
    let ks = parse_list_of_symbol_strings(params)?;
    if ks.len() != args.len() {
        return Err(GError::Reason(format!(
            "expected {} params, got {}",
            ks.len(),
            args.len()
        )));
    }
    let vs = eval_forms(args, outer_env)?;
    let mut data: HashMap<String, Expression> = HashMap::new();
    for (k, v) in ks.iter().zip(vs.iter()) {
        data.insert(k.clone(), v.clone());
    }

    Ok(Environment {
        data,
        outer: Some(outer_env),
    })
}

fn eval_forms(args: &[Expression], env: &mut Environment) -> Result<Vec<Expression>, GError> {
    args.iter().map(|x| eval(x, env)).collect()
}

fn parse_list_of_symbol_strings(params: Rc<Expression>) -> Result<Vec<String>, GError> {
    let list = match params.as_ref() {
        Expression::List(s) => Ok(s.clone()),
        _ => Err(GError::Reason(format!("expected params to be a list"))),
    }?;
    list.iter()
        .map(|x| match x {
            Expression::Symbol(s) => Ok(s.clone()),
            _ => Err(GError::Reason(format!(
                "expected symbol in the argument list"
            ))),
        })
        .collect()
}

fn env_get(key: &str, env: &Environment) -> Option<Expression> {
    match env.data.get(key) {
        Some(exp) => Some(exp.clone()),
        None => match env.outer {
            Some(outer_env) => env_get(key, &outer_env),
            None => None,
        },
    }
}

fn eval(exp: &Expression, env: &mut Environment) -> Result<Expression, GError> {
    match exp {
        Expression::Bool(_) => Ok(exp.clone()),
        Expression::Symbol(k) => env_get(&k, env)
            .ok_or(GError::Reason(format!("unexpected symbol k={}", k)))
            .map(|x| x.clone()),
        Expression::Number(_a) => Ok(exp.clone()),
        Expression::List(list) => {
            let first_form = list
                .first()
                .ok_or(GError::Reason("expected a non-empty list".to_string()))?;
            let arg_forms = &list[1..];

            match eval_built_in_form(first_form, arg_forms, env) {
                Some(built_in_res) => built_in_res,
                None => {
                    let first_eval = eval(first_form, env)?;
                    match first_eval {
                        Expression::Func(f) => {
                            let args_eval = arg_forms
                                .iter()
                                .map(|x| eval(x, env))
                                .collect::<Result<Vec<Expression>, GError>>();
                            f(&args_eval?)
                        }
                        Expression::Lambda(lambda) => {
                            // ->  New
                            let new_env = &mut env_for_lambda(lambda.params, arg_forms, env)?;
                            eval(&lambda.body, new_env)
                        }
                        _ => Err(GError::Reason("first form must be a function".to_string())),
                    }
                }
            }
        }
        Expression::Func(_) => Err(GError::Reason("unexpected form".to_string())),
        Expression::String(_) => Ok(exp.clone()),
        _ => Err(GError::Reason("not supported type.".to_string())),
    }
}

fn eval_if_args(args: &[Expression], env: &mut Environment) -> Result<Expression, GError> {
    let test_form = args
        .first()
        .ok_or(GError::Reason("expected test form".to_string()))?;
    let test_eval = eval(test_form, env)?;
    match test_eval {
        Expression::Bool(b) => {
            let form_idx = if b { 1 } else { 2 };
            let res_form = args
                .get(form_idx)
                .ok_or(GError::Reason(format!("expected form idx={}", form_idx)))?;
            let res_eval = eval(res_form, env);
            res_eval
        }
        _ => Err(GError::Reason(format!(
            "unexpected test form='{}'",
            test_form.to_string()
        ))),
    }
}

fn eval_def_args(args: &[Expression], env: &mut Environment) -> Result<Expression, GError> {
    let var_exp = args
        .first()
        .ok_or(GError::Reason(format!("unexepceted string for var")))?;

    let val_res = args
        .get(1)
        .ok_or(GError::Reason(format!("expected second param.")))?;
    let evaled_val = eval(val_res, env)?;

    match var_exp {
        Expression::Symbol(ref var_name) => {
            env.data.insert(var_name.clone(), evaled_val);
            Ok(var_exp.clone())
        }
        _ => Err(GError::Reason(format!("unexpected var name"))),
    }
}

fn parse_eval(expr: String, env: &mut Environment) -> Result<Expression, GError> {
    let (parsed_exp, _) = parse(&tokenize(expr))?;
    let evaled_exp = eval(&parsed_exp, env)?;
    Ok(evaled_exp)
}

fn slurp_expr() -> String {
    let mut expr = String::new();
    std::io::stdin()
        .read_line(&mut expr)
        .expect("Failed to read line");
    expr
}

pub fn run_repl() {
    let env = &mut default_env();
    loop {
        print!("glisp > ");
        let _ = std::io::stdout().flush();
        let expr = slurp_expr();
        match parse_eval(expr, env) {
            Ok(res) => println!("// => {}", res),
            Err(e) => match e {
                GError::Reason(msg) => println!("// => {}", msg),
            },
        }
    }
}

fn eval_quote_args(args: &[Expression]) -> Result<Expression, GError> {
    let _fst = args
        .first()
        .ok_or(GError::Reason(format!("unexpected args form")))?;
    let mut retfst = vec![Expression::Symbol("quote".to_owned())];
    retfst.extend_from_slice(args);
    Ok(Expression::List(retfst))
}

fn eval_atom_args(args: &[Expression], env: &mut Environment) -> Result<Expression, GError> {
    let fst = eval(
        args.first()
            .ok_or(GError::Reason(format!("unexpected args form")))?,
        env,
    )?;
    match fst {
        Expression::Symbol(_) => Ok(Expression::Bool(true)),
        Expression::Number(_) => Ok(Expression::Bool(true)),
        Expression::Func(_) => Ok(Expression::Bool(true)),
        Expression::Bool(_) => Ok(Expression::Bool(true)),
        Expression::String(_) => Ok(Expression::Bool(true)),
        Expression::List(a) => {
            let _fst = if let Some(e) = a.get(0) {
                e
            } else {
                return Ok(Expression::Bool(true));
            }
            .to_string();
            if _fst == *"quote" {
                Ok(Expression::Bool(false))
            } else {
                Ok(Expression::Bool(true))
            }
        }
        _ => Ok(Expression::Bool(false)),
    }
}

//TODO: Parameter limit macro
fn eval_eq_args(args: &[Expression], env: &mut Environment) -> Result<Expression, GError> {
    let (fst, snd) = (
        args.first()
            .ok_or(GError::Reason(format!("eq can only have 2 args")))?,
        args.get(1)
            .ok_or(GError::Reason(format!("eq can only have 2 args")))?,
    );

    if eval(fst, env)?.to_string() == eval(snd, env)?.to_string() {
        Ok(Expression::Bool(true))
    } else {
        Ok(Expression::Bool(false))
    }
}

fn eval_car_args(args: &[Expression]) -> Result<Expression, GError> {
    let fst = args.first().ok_or(GError::Reason(format!("car: Error")))?;
    let lst = match fst {
        Expression::List(a) => a,
        _ => return Err(GError::Reason(format!("car can only result a static list"))),
    };
    let lstfst = lst.get(0);
    if let Some(a) = lstfst {
        if a.to_string() == "quote" {
            return Ok(if let Some(_a) = lst.get(1) {
                _a.clone()
            } else {
                return Err(GError::Reason(format!("car can only result a static list")));
            });
        } else {
            return Err(GError::Reason(format!("car can only result a static list")));
        }
    };
    return Err(GError::Reason(format!("car can only result a static list")));
}

fn eval_cdr_args(args: &[Expression]) -> Result<Expression, GError> {
    let fst = args.first().ok_or(GError::Reason(format!("car: Error")))?;
    let lst = match fst {
        Expression::List(a) => a,
        _ => return Err(GError::Reason(format!("cdr can only result a static list"))),
    };
    if lst.len() < 4 {
        return Err(GError::Reason(format!("cdr: the len of list must >= 4")));
    }
    let lstfst = lst.get(0);
    if let Some(a) = lstfst {
        if a.to_string() == "quote" {
            let mut retfst = vec![Expression::Symbol("quote".to_owned())];
            retfst.extend_from_slice(&lst[2..]);
            return Ok(Expression::List(retfst));
        } else {
            return Err(GError::Reason(format!("cdr can only result a static list")));
        }
    };
    return Err(GError::Reason(format!("cdr can only result a static list")));
}

fn eval_cons_args(args: &[Expression]) -> Result<Expression, GError> {
    if args.len() < 2 {
        return Err(GError::Reason(format!("cons: Error")));
    }

    let mut lst1 = match args[0].clone() {
        Expression::List(a) => a,
        _ => {
            return Err(GError::Reason(format!(
                "cons can only result a static list"
            )))
        }
    };

    if lst1.remove(0).to_string() != "quote" {
        return Err(GError::Reason(format!(
            "cons can only result a static list"
        )));
    }

    let mut lst2 = match args[1].clone() {
        Expression::List(a) => a,
        _ => {
            return Err(GError::Reason(format!(
                "cons can only result a static list"
            )))
        }
    };

    if lst2.remove(0).to_string() != "quote" {
        return Err(GError::Reason(format!(
            "cons can only result a static list"
        )));
    }

    lst1.extend(lst2);

    Ok(Expression::List(lst1))
}

fn eval_cond_args(args: &[Expression], env: &mut Environment) -> Result<Expression, GError> {
    if args.len() < 2 {
        return Err(GError::Reason(format!("cond: Error1")));
    }

    let mut i = 0;
    loop {
        if i >= args.len() {
            return Err(GError::Reason(format!("cond: Error2")));
        }

        let v = match eval(&args[i * 2].clone(), env) {
            Ok(a) => a,
            _ => return Err(GError::Reason(format!("cond: Error3"))),
        };
        match v {
            Expression::Bool(true) => return eval(&args[i * 2 + 1].clone(), env),
            _ => (),
        }
        i += 1;
    }
}

fn eval_built_in_form(
    exp: &Expression,
    other_args: &[Expression],
    env: &mut Environment,
) -> Option<Result<Expression, GError>> {
    match exp {
        Expression::Symbol(symbol) => match symbol.as_ref() {
            "if" => Some(eval_if_args(other_args, env)),
            "def" => Some(eval_def_args(other_args, env)),
            "lambda" => Some(eval_lambda_args(other_args)),
            "quote" => Some(eval_quote_args(other_args)),
            "atom" => Some(eval_atom_args(other_args, env)),
            "eq" => Some(eval_eq_args(other_args, env)),
            "car" => Some(eval_car_args(other_args)),
            "cdr" => Some(eval_cdr_args(other_args)),
            "cons" => Some(eval_cons_args(other_args)),
            "cond" => Some(eval_cond_args(other_args, env)),
            _ => None,
        },
        _ => None,
    }
}
