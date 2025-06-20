use std::cell::RefCell;
use std::collections::HashMap;

use hydro_lang::ir::{HydroLeaf, HydroNode, traverse_dfir};
use syn::visit::Visit;

type TupleIndex = Vec<usize>; // Ex: [0,1] represents a.0.1

#[derive(Debug, Clone, Default, Eq)]
pub struct Tuple {
    dependency: Option<TupleIndex>, // Input tuple index this tuple is equal to, if any
    fields: HashMap<usize, Box<Tuple>>, // Fields 1 layer deep
}

impl Tuple {
    fn is_empty(&self) -> bool {
        self.dependency.is_none() && self.fields.is_empty()
    }

    fn create_child(&mut self, index: TupleIndex) -> &mut Tuple {
        let mut child = self;
        for i in index {
            child = &mut **child
                .fields
                .entry(i)
                .or_insert_with(|| Box::new(Tuple::default()));
        }
        child
    }

    pub fn set_dependencies(
        &mut self,
        index: &TupleIndex,
        mut rhs: &Tuple,
        rhs_index: &TupleIndex,
    ) {
        // Navigate to the index for the RHS
        for (i, tuple_index) in rhs_index.iter().enumerate() {
            if let Some(child) = rhs.fields.get(tuple_index) {
                rhs = child.as_ref();
            } else if let Some(dependency) = &rhs.dependency {
                // RHS has a broader dependency, extend it with the remaining index
                let mut specific_dependency = dependency.clone();
                specific_dependency.extend(rhs_index);
                // Create a child if necessary and set the dependency
                let child = self.create_child(index.clone());
                child.dependency = Some(specific_dependency);
                return;
            } else {
                // RHS has no dependency, exit
                return;
            }
        }

        // Create a child if necessary and copy everything from the RHS
        let child = self.create_child(index.clone());
        child.dependency = rhs.dependency.clone();
        child.fields = rhs.fields.clone();
    }

    pub fn set_dependency(&mut self, index: &TupleIndex, input_tuple_index: TupleIndex) {
        let child = self.create_child(index.clone());
        child.dependency = Some(input_tuple_index);
    }
}

impl PartialEq for Tuple {
    fn eq(&self, other: &Self) -> bool {
        self.dependency == other.dependency && self.fields == other.fields
    }
}

// Find whether a tuple's usage (Ex: a.0.1) references an existing var (Ex: a), and if so, calculate the new TupleIndex
#[derive(Default)]
struct TupleUseRhs {
    existing_dependencies: HashMap<syn::Ident, Tuple>,
    rhs_tuple: Tuple,
    tuple_position_in_paren: TupleIndex, /* Used to track where we are in the tuple as we recurse. Ex: ((a, b), c) -> [0, 1] for b */
    tuple_field_index: TupleIndex, /* Used to track the index of the tuple recursively. Ex: a.0.1 -> [0, 1] */
}

impl Visit<'_> for TupleUseRhs {
    fn visit_expr_path(&mut self, path: &syn::ExprPath) {
        if let Some(ident) = path.path.get_ident() {
            // Base path matches an Ident that has an existing dependency in one of its fields
            if let Some(existing_dependency) = self.existing_dependencies.get(ident) {
                self.rhs_tuple.set_dependencies(
                    &self.tuple_position_in_paren,
                    existing_dependency,
                    &self.tuple_field_index,
                );
            }
        }
    }

    fn visit_expr_field(&mut self, expr: &syn::ExprField) {
        // Find the ident of the rightmost field
        let index = match &expr.member {
            syn::Member::Named(ident) => {
                panic!(
                    "Partitioning analysis currently supports only tuples, not structs. Found a named field on a struct: {:?}",
                    ident
                );
            }
            syn::Member::Unnamed(index) => index.index as usize,
        };

        // Keep going left until we get to the root
        self.tuple_field_index.insert(0, index);
        self.visit_expr(expr.base.as_ref());
    }

    fn visit_expr_tuple(&mut self, tuple: &syn::ExprTuple) {
        // Recursively visit elems, in case we have nested tuples
        let pre_recursion_index = self.tuple_position_in_paren.clone();
        for (i, elem) in tuple.elems.iter().enumerate() {
            self.tuple_position_in_paren = pre_recursion_index.clone();
            self.tuple_position_in_paren.push(i);
            // Reset field index
            self.tuple_field_index.clear();
            self.visit_expr(elem);
        }
    }
}

// Create a mapping from Ident to tuple indices (Note: Not necessarily input tuple indices)
// For example, (a, (b, c)) -> { a: [0], b: [1, 0], c: [1, 1] }
#[derive(Default)]
struct TupleDeclareLhs {
    lhs_tuple: HashMap<syn::Ident, TupleIndex>,
    tuple_index: TupleIndex, // Internal, used to track the index of the tuple recursively
}

impl TupleDeclareLhs {
    fn into_tuples(&self) -> HashMap<syn::Ident, Tuple> {
        let mut tuples = HashMap::new();
        for (ident, index) in &self.lhs_tuple {
            let mut tuple = Tuple::default();
            tuple.dependency = Some(index.clone());
            tuples.insert(ident.clone(), tuple);
        }
        tuples
    }
}

impl Visit<'_> for TupleDeclareLhs {
    fn visit_pat(&mut self, pat: &syn::Pat) {
        match pat {
            syn::Pat::Ident(ident) => {
                self.lhs_tuple
                    .insert(ident.ident.clone(), self.tuple_index.clone());
            }
            syn::Pat::Tuple(tuple) => {
                // Recursively visit elems, in case we have nested tuples
                let pre_recursion_index = self.tuple_index.clone();
                for (i, elem) in tuple.elems.iter().enumerate() {
                    self.tuple_index = pre_recursion_index.clone();
                    self.tuple_index.push(i);
                    self.visit_pat(elem);
                }
            }
            _ => {
                panic!(
                    "TupleDeclareLhs does not support this LHS pattern: {:?}",
                    pat
                );
            }
        }
    }
}

#[derive(Default)]
struct EqualityAnalysis {
    output_dependencies: Tuple,
    dependencies: HashMap<syn::Ident, Tuple>,
}

impl Visit<'_> for EqualityAnalysis {
    fn visit_expr_return(&mut self, _: &syn::ExprReturn) {
        panic!("Partitioning analysis does not support return.");
    }

    fn visit_stmt(&mut self, stmt: &syn::Stmt) {
        match stmt {
            syn::Stmt::Local(local) => {
                // Analyze LHS
                let mut input_analysis = TupleDeclareLhs::default();
                input_analysis.visit_pat(&local.pat);

                // Analyze RHS
                let mut analysis = TupleUseRhs::default();
                if let Some(init) = local.init.as_ref() {
                    // See if RHS is a direct match for an existing dependency
                    analysis.existing_dependencies = self.dependencies.clone();
                    analysis.visit_expr(init.expr.as_ref());
                }

                // Set dependencies from LHS to RHS
                for (lhs, tuple_index) in input_analysis.lhs_tuple.iter() {
                    let mut tuple = Tuple::default();
                    tuple.set_dependencies(tuple_index, &analysis.rhs_tuple, tuple_index);
                    if tuple.is_empty() {
                        // No RHS dependency found, delete LHS if it exists (it shadows any previous dependency)
                        self.dependencies.remove(lhs);
                    } else {
                        // Found a match, insert into dependencies
                        println!("Found dependency: {} {:?} = {:?}", lhs, tuple_index, tuple);
                        self.dependencies.insert(lhs.clone(), tuple);
                    }
                }
            }
            _ => {}
        }
    }

    fn visit_expr_block(&mut self, block: &syn::ExprBlock) {
        for (i, stmt) in block.block.stmts.iter().enumerate() {
            self.visit_stmt(stmt);

            if i == block.block.stmts.len() - 1 {
                // If this is the last statement, it is the output if there is no semicolon
                if let syn::Stmt::Expr(expr, semicolon) = stmt {
                    if semicolon.is_none() {
                        // Output only exists if there is no semicolon
                        let mut analysis = TupleUseRhs::default();
                        analysis.existing_dependencies = self.dependencies.clone();
                        analysis.visit_expr(expr);

                        self.output_dependencies = analysis.rhs_tuple;
                        println!("Output dependency: {:?}", self.output_dependencies);
                    }
                }
            }
        }
    }
}

#[derive(Default)]
pub struct AnalyzeClosure {
    found_closure: bool, // Used to avoid executing visit_pat on anything but the function body
    pub output_dependencies: Tuple,
}

impl Visit<'_> for AnalyzeClosure {
    fn visit_expr_closure(&mut self, closure: &syn::ExprClosure) {
        if self.found_closure {
            panic!(
                "Multiple top-level closures found in a single Expr during partitioning analysis, likely due to running analysis over a function such as reduce."
            );
        }

        // Find all input vars
        self.found_closure = true;
        if closure.inputs.len() > 1 {
            panic!(
                "Partitioning analysis does not currently support closures with multiple inputs (such as reduce): {:?}.",
                closure
            );
        }
        let mut input_analysis = TupleDeclareLhs::default();
        input_analysis.visit_pat(&closure.inputs[0]);
        println!(
            "Input idents to tuple indices: {:?}",
            input_analysis.lhs_tuple
        );

        // Perform dependency analysis on the body
        let mut analyzer = EqualityAnalysis::default();
        analyzer.dependencies = input_analysis.into_tuples();
        analyzer.visit_expr(&closure.body);
        self.output_dependencies = analyzer.output_dependencies;

        println!(
            "Closure output dependencies: {:?}",
            self.output_dependencies
        );
    }
}

pub struct PartitioningMetadata {
    pub output_dependencies: HashMap<usize, Tuple>, /* Map from stmt_id to output tuple dependencies */
}

fn partition_analysis_leaf(
    leaf: &mut HydroLeaf,
    next_stmt_id: &mut usize,
    metadata: &RefCell<PartitioningMetadata>,
) {
    let mut analyzer = AnalyzeClosure::default();
    leaf.visit_debug_expr(|debug_expr| {
        analyzer.visit_expr(&debug_expr.0);
    });
    metadata
        .borrow_mut()
        .output_dependencies
        .insert(*next_stmt_id, analyzer.output_dependencies.clone());
}

fn partition_analysis_node(
    node: &mut HydroNode,
    next_stmt_id: &mut usize,
    metadata: &RefCell<PartitioningMetadata>,
) {
    let mut analyzer = AnalyzeClosure::default();
    node.visit_debug_expr(|debug_expr| {
        analyzer.visit_expr(&debug_expr.0);
    });
    metadata
        .borrow_mut()
        .output_dependencies
        .insert(*next_stmt_id, analyzer.output_dependencies.clone());
}

pub fn partition_analysis(ir: &mut [HydroLeaf]) -> PartitioningMetadata {
    let partitioning_metadata = RefCell::new(PartitioningMetadata {
        output_dependencies: HashMap::new(),
    });
    traverse_dfir(
        ir,
        |leaf, next_stmt_id| {
            partition_analysis_leaf(leaf, next_stmt_id, &partitioning_metadata);
        },
        |node, next_stmt_id| {
            partition_analysis_node(node, next_stmt_id, &partitioning_metadata);
        },
    );

    partitioning_metadata.into_inner()
}

#[cfg(test)]
mod tests {
    use hydro_lang::deploy::DeployRuntime;
    use hydro_lang::ir::deep_clone;
    use hydro_lang::{FlowBuilder, Location};
    use stageleft::{RuntimeData, q};

    use crate::partition_analysis::{Tuple, partition_analysis};

    fn verify_abcde_tuple(builder: FlowBuilder<'_>) {
        let built = builder.with_default_optimize::<DeployRuntime>();
        let mut ir = deep_clone(built.ir());
        let metadata = partition_analysis(&mut ir);

        let mut expected_output_dependency = Tuple::default();
        expected_output_dependency.set_dependency(&vec![0], vec![0]);
        expected_output_dependency.set_dependency(&vec![1], vec![1]);
        expected_output_dependency.set_dependency(&vec![2, 0], vec![2, 0]);
        expected_output_dependency.set_dependency(&vec![2, 1, 0], vec![2, 1, 0]);
        expected_output_dependency.set_dependency(&vec![3], vec![3]);
        assert_eq!(
            metadata.output_dependencies.get(&1),
            Some(&expected_output_dependency)
        );

        let _ = built.compile(&RuntimeData::new("FAKE"));
    }

    #[test]
    fn test_tuple_input_assignment() {
        let builder = FlowBuilder::new();
        let cluster = builder.cluster::<()>();
        cluster
            .source_iter(q!([(1, 2, (3, (4,)), 5)]))
            .map(q!(|(a, b, (c, (d,)), e)| { (a, b, (c, (d,)), e) }))
            .for_each(q!(|(a, b, (c, (d,)), e)| {
                println!("a: {}, b: {}, c: {}, d: {}, e: {}", a, b, c, d, e);
            }));
        verify_abcde_tuple(builder);
    }

    #[test]
    fn test_tuple_input_implicit_nesting() {
        let builder = FlowBuilder::new();
        let cluster = builder.cluster::<()>();
        cluster
            .source_iter(q!([(1, 2, (3, (4,)), 5)]))
            .map(q!(|(a, b, cd, e)| { (a, b, (cd.0, (cd.1.0,)), e) }))
            .for_each(q!(|(a, b, (c, (d,)), e)| {
                println!("a: {}, b: {}, c: {}, d: {}, e: {}", a, b, c, d, e);
            }));
        verify_abcde_tuple(builder);
    }

    #[test]
    fn test_tuple_assignment() {
        let builder = FlowBuilder::new();
        let cluster = builder.cluster::<()>();
        cluster
            .source_iter(q!([(1, 2, (3, (4,)), 5)]))
            .map(q!(|(a, b, (c, (d,)), e)| {
                let f = c;
                (a, b, (f, (d,)), e)
            }))
            .for_each(q!(|(a, b, (c, (d,)), e)| {
                println!("a: {}, b: {}, c: {}, d: {}, e: {}", a, b, c, d, e);
            }));
        verify_abcde_tuple(builder);
    }

    #[test]
    fn test_tuple_creation() {
        let builder = FlowBuilder::new();
        let cluster = builder.cluster::<()>();
        cluster
            .source_iter(q!([(1, 2, (3, (4,)), 5)]))
            .map(q!(|(a, b, (c, (d,)), e)| {
                let f = (c, (d,));
                (a, b, (f.0, (f.1.0,)), e)
            }))
            .for_each(q!(|(a, b, (c, (d,)), e)| {
                println!("a: {}, b: {}, c: {}, d: {}, e: {}", a, b, c, d, e);
            }));
        verify_abcde_tuple(builder);
    }

    #[test]
    fn test_tuple_output_implicit_nesting() {
        let builder = FlowBuilder::new();
        let cluster = builder.cluster::<()>();
        cluster
            .source_iter(q!([(1, 2, (3, (4,)), 5)]))
            .map(q!(|(a, b, (c, (d,)), e)| {
                let f = (c, (d,));
                (a, b, f, e)
            }))
            .for_each(q!(|(a, b, (c, (d,)), e)| {
                println!("a: {}, b: {}, c: {}, d: {}, e: {}", a, b, c, d, e);
            }));
        verify_abcde_tuple(builder);
    }
}
