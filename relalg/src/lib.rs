#![allow(dead_code)]
#![allow(clippy::iter_with_drain)]
#![allow(clippy::explicit_auto_deref)]
use anyhow::bail;
use sexp::Sexp;

mod codegen;
mod runtime;
mod sexp;

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum Datum {
    Int(i64),
    String(String),
    Bool(bool),
}

impl Datum {
    fn unwrap_int(self) -> i64 {
        if let Datum::Int(i) = self {
            i
        } else {
            panic!("was not int")
        }
    }

    pub fn is_true(self) -> bool {
        matches!(self, Datum::Bool(true))
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum ScalarExpr {
    Literal(Datum),
    ColRef(usize),
    Eq(Box<ScalarExpr>, Box<ScalarExpr>),
    Plus(Box<ScalarExpr>, Box<ScalarExpr>),
}

impl ScalarExpr {
    pub fn eval(&self, data: &[Datum]) -> Datum {
        match self {
            ScalarExpr::Literal(d) => d.clone(),
            ScalarExpr::ColRef(u) => data[*u].clone(),
            ScalarExpr::Eq(a, b) => {
                let a = a.eval(data);
                let b = b.eval(data);
                Datum::Bool(a == b)
            }
            ScalarExpr::Plus(a, b) => {
                let a = a.eval(data).unwrap_int();
                let b = b.eval(data).unwrap_int();
                Datum::Int(a + b)
            }
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
enum RelExpr {
    Values(Vec<Vec<ScalarExpr>>),
    Filter(Vec<ScalarExpr>, Box<RelExpr>),
    Project(Vec<ScalarExpr>, Box<RelExpr>),
}

fn parse_scalar(s: Sexp) -> anyhow::Result<ScalarExpr> {
    match s {
        Sexp::Atom(s) => {
            if s.strip_prefix('@').is_some() {
                Ok(ScalarExpr::ColRef(s[1..].parse::<usize>()?))
            } else {
                Ok(ScalarExpr::Literal(Datum::Int(s.parse::<i64>()?)))
            }
        }
        Sexp::String(s) => Ok(ScalarExpr::Literal(Datum::String(s))),
        Sexp::List(list, _) => {
            if list.is_empty() {
                bail!("empty list is not a relexpr");
            }
            match list[0].clone().expect_atom()?.as_str() {
                "=" => {
                    if list.len() < 3 {
                        bail!("= requires two arguments");
                    }
                    Ok(ScalarExpr::Eq(
                        Box::new(parse_scalar(list[1].clone())?),
                        Box::new(parse_scalar(list[2].clone())?),
                    ))
                }
                "+" => {
                    if list.len() < 3 {
                        bail!("+ requires two arguments");
                    }
                    Ok(ScalarExpr::Plus(
                        Box::new(parse_scalar(list[1].clone())?),
                        Box::new(parse_scalar(list[2].clone())?),
                    ))
                }
                v => bail!("{} is not a scalar operator", v),
            }
        }
    }
}

fn parse_relexpr(s: Sexp) -> anyhow::Result<RelExpr> {
    let (list, _) = s.expect_list()?;
    if list.is_empty() {
        bail!("empty list is not a relexpr");
    }
    match list[0].clone().expect_atom()?.as_str() {
        "values" => {
            if list.len() < 2 {
                bail!("values requires an argument");
            }
            let (values, _) = list[1].clone().expect_list()?;
            let mut rows = Vec::new();
            for r in values {
                let (row, _) = r.expect_list()?;
                let mut parsed_row = Vec::new();
                for d in row {
                    parsed_row.push(parse_scalar(d)?);
                }
                rows.push(parsed_row);
            }
            Ok(RelExpr::Values(rows))
        }
        "project" => {
            if list.len() < 3 {
                bail!("filter requires two arguments");
            }
            let (exprs_sexp, _) = list[1].clone().expect_list()?;
            let exprs = exprs_sexp
                .into_iter()
                .map(parse_scalar)
                .collect::<anyhow::Result<Vec<ScalarExpr>>>()?;

            Ok(RelExpr::Project(
                exprs,
                Box::new(parse_relexpr(list[2].clone())?),
            ))
        }
        "filter" => {
            if list.len() < 3 {
                bail!("filter requires two arguments");
            }
            let (filters_sexp, _) = list[1].clone().expect_list()?;
            let filters = filters_sexp
                .into_iter()
                .map(parse_scalar)
                .collect::<anyhow::Result<Vec<ScalarExpr>>>()?;

            Ok(RelExpr::Filter(
                filters,
                Box::new(parse_relexpr(list[2].clone())?),
            ))
        }
        v => bail!("{} is not a relational operator", v),
    }
}

#[cfg(test)]
mod tests {
    use crate::codegen::generate_dataflow;
    use crate::parse_relexpr;
    use crate::runtime::run_dataflow;
    use crate::sexp::Sexp;

    #[test]
    fn datadriven_tests() {
        datadriven::walk("testdata/", |t| {
            t.run(|test_case| match test_case.directive.as_str() {
                "compile" => {
                    let sexps = Sexp::parse(test_case.input.clone()).unwrap();
                    let sexp = sexps[0].clone();
                    let rel_expr = parse_relexpr(sexp).unwrap();
                    let output = generate_dataflow(rel_expr);
                    format!("{}\n", output)
                }
                "build" => {
                    let sexps = Sexp::parse(test_case.input.clone()).unwrap();
                    let sexp = sexps[0].clone();
                    let rel_expr = parse_relexpr(sexp);
                    format!("{:?}\n", rel_expr)
                }
                "run" => {
                    let sexps = Sexp::parse(test_case.input.clone()).unwrap();
                    let sexp = sexps[0].clone();
                    let rel_expr = parse_relexpr(sexp).unwrap();
                    let output = run_dataflow(rel_expr);
                    format!("{:?}\n", output)
                }
                _ => "unhandled directive\n".into(),
            })
        });
    }
}
