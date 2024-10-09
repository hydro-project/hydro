use std::collections::HashSet;

use stageleft::*;

use crate::ir::{HfPlusLeaf, HfPlusNode, SeenTees};

/// Structure for tracking expressions known to have particular algebraic properties.
///
/// # Schema
///
/// Each field in this struct corresponds to an algebraic property, and contains the list of
/// expressions that satisfy the property. Currently only `commutative`.
///
/// # Interface
///
/// "Tag" an expression with a property and it will add it to that table. For example, [`Self::add_commutative_tag`].
/// Can also run a check to see if an expression satisfies a property.
#[derive(Default)]
pub struct PropertyDatabase {
    commutative: HashSet<syn::Expr>,
}

/// Allows us to convert the hydroflow datatype for folds to a binary operation for the algebra
/// property tests.
#[allow(clippy::allow_attributes, dead_code, reason = "staged programming")]
fn convert_hf_to_binary<I, A: Default, F: Fn(&mut A, I)>(f: F) -> impl Fn(I, I) -> A {
    move |a, b| {
        let mut acc = Default::default();
        f(&mut acc, a);
        f(&mut acc, b);
        acc
    }
}

impl PropertyDatabase {
    /// Tags the expression as commutative.
    pub fn add_commutative_tag<'a, I, A, F: Fn(&mut A, I), Q: Quoted<'a, F> + Clone>(
        &mut self,
        expr: Q,
    ) -> Q {
        let expr_clone = expr.clone();
        self.commutative.insert(expr_clone.splice_untyped());
        expr
    }

    pub fn is_tagged_commutative(&self, expr: &syn::Expr) -> bool {
        self.commutative.contains(expr)
    }
}

// Dataflow graph optimization rewrite rules based on algebraic property tags
// TODO add a test that verifies the space of possible graphs after rewrites is correct for each property

fn properties_optimize_node(
    node: &mut HfPlusNode,
    db: &PropertyDatabase,
    seen_tees: &mut SeenTees,
) {
    node.transform_children(
        |node, seen_tees| properties_optimize_node(node, db, seen_tees),
        seen_tees,
    );
    match node {
        HfPlusNode::ReduceKeyed { f, .. } if db.is_tagged_commutative(&f.0) => {
            dbg!("IDENTIFIED COMMUTATIVE OPTIMIZATION for {:?}", &f);
        }
        _ => {}
    }
}

pub fn properties_optimize(ir: Vec<HfPlusLeaf>, db: &PropertyDatabase) -> Vec<HfPlusLeaf> {
    let mut seen_tees = Default::default();
    ir.into_iter()
        .map(|l| {
            l.transform_children(
                |node, seen_tees| properties_optimize_node(node, db, seen_tees),
                &mut seen_tees,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deploy::SingleProcessGraph;
    use crate::location::Location;
    use crate::FlowBuilder;

    #[test]
    fn test_property_database() {
        let mut db = PropertyDatabase::default();

        assert!(!db.is_tagged_commutative(&(q!(|a: &mut i32, b: i32| *a += b).splice_untyped())));

        let _ = db.add_commutative_tag(q!(|a: &mut i32, b: i32| *a += b));

        assert!(db.is_tagged_commutative(&(q!(|a: &mut i32, b: i32| *a += b).splice_untyped())));
    }

    #[test]
    fn test_property_optimized() {
        let flow = FlowBuilder::new();
        let mut database = PropertyDatabase::default();

        let process = flow.process::<()>();

        let counter_func = q!(|count: &mut i32, _| *count += 1);
        let _ = database.add_commutative_tag(counter_func);

        process
            .source_iter(q!(vec![]))
            .map(q!(|string: String| (string, ())))
            .tick_batch()
            .fold_keyed(q!(|| 0), counter_func)
            .all_ticks()
            .for_each(q!(|(string, count)| println!("{}: {}", string, count)));

        let built = flow
            .finalize()
            .optimize_with(|ir| properties_optimize(ir, &database))
            .with_default_optimize();

        insta::assert_debug_snapshot!(built.ir());

        let _ = built.compile_no_network::<SingleProcessGraph>();
    }
}
