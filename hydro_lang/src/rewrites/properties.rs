use std::collections::HashSet;

use stageleft::*;

use crate::ir::{HydroLeaf, HydroNode, transform_bottom_up};

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

impl PropertyDatabase {
    /// Tags the expression as commutative.
    pub fn add_commutative_tag<
        'a,
        I,
        A,
        F: Fn(&mut A, I),
        Ctx,
        Q: QuotedWithContext<'a, F, Ctx> + Clone,
    >(
        &mut self,
        expr: Q,
        ctx: &Ctx,
    ) -> Q {
        let expr_clone = expr.clone();
        self.commutative.insert(expr_clone.splice_untyped_ctx(ctx));
        expr
    }

    pub fn is_tagged_commutative(&self, expr: &syn::Expr) -> bool {
        self.commutative.contains(expr)
    }
}

// Dataflow graph optimization rewrite rules based on algebraic property tags
// TODO add a test that verifies the space of possible graphs after rewrites is correct for each property

fn properties_optimize_node(node: &mut HydroNode, db: &mut PropertyDatabase) {
    match node {
        HydroNode::ReduceKeyed { f, .. } if db.is_tagged_commutative(&f.0) => {
            dbg!("IDENTIFIED COMMUTATIVE OPTIMIZATION for {:?}", &f);
        }
        _ => {}
    }
}

pub fn properties_optimize(ir: &mut [HydroLeaf], db: &mut PropertyDatabase) {
    transform_bottom_up(
        ir,
        &mut |_| (),
        &mut |node| properties_optimize_node(node, db),
        false,
    );
}

#[cfg(stageleft_runtime)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::FlowBuilder;
    use crate::deploy::HydroDeploy;
    use crate::location::Location;

    #[test]
    fn test_property_database() {
        let mut db = PropertyDatabase::default();

        assert!(
            !db.is_tagged_commutative(&(q!(|a: &mut i32, b: i32| *a += b).splice_untyped_ctx(&())))
        );

        let _ = db.add_commutative_tag(q!(|a: &mut i32, b: i32| *a += b), &());

        assert!(
            db.is_tagged_commutative(&(q!(|a: &mut i32, b: i32| *a += b).splice_untyped_ctx(&())))
        );
    }

    #[test]
    fn test_property_optimized() {
        let flow = FlowBuilder::new();
        let mut database = PropertyDatabase::default();

        let process = flow.process::<()>();
        let tick = process.tick();

        let counter_func = q!(|count: &mut i32, _| *count += 1);
        let _ = database.add_commutative_tag(counter_func, &tick);

        unsafe {
            process
                .source_iter(q!(vec![]))
                .map(q!(|string: String| (string, ())))
                .tick_batch(&tick)
        }
        .into_keyed()
        .fold(q!(|| 0), counter_func)
        .entries()
        .all_ticks()
        .for_each(q!(|(string, count)| println!("{}: {}", string, count)));

        let built = flow
            .optimize_with(|ir| properties_optimize(ir, &mut database))
            .with_default_optimize::<HydroDeploy>();

        insta::assert_debug_snapshot!(built.ir());

        let _ = built.compile_no_network();
    }
}
