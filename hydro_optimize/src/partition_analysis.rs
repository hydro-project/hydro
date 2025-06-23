use std::cell::RefCell;
use std::collections::HashMap;

use hydro_lang::ir::{HydroLeaf, HydroNode, traverse_dfir};
use syn::visit::Visit;

type StructOrTupleIndex = Vec<String>; // Ex: ["a", "b"] represents x.a.b

#[derive(Debug, Clone, Default, Eq)]
pub struct StructOrTuple {
    dependency: Option<StructOrTupleIndex>, // Input tuple index this tuple is equal to, if any
    fields: HashMap<String, Box<StructOrTuple>>, // Fields 1 layer deep
}

impl StructOrTuple {
    fn is_empty(&self) -> bool {
        self.dependency.is_none() && self.fields.is_empty()
    }

    fn create_child(&mut self, index: StructOrTupleIndex) -> &mut StructOrTuple {
        let mut child = self;
        for i in index {
            child = &mut **child
                .fields
                .entry(i)
                .or_insert_with(|| Box::new(StructOrTuple::default()));
        }
        child
    }

    pub fn set_dependencies(
        &mut self,
        index: &StructOrTupleIndex,
        mut rhs: &StructOrTuple,
        rhs_index: &StructOrTupleIndex,
    ) {
        // Navigate to the index for the RHS
        for tuple_index in rhs_index {
            if let Some(child) = rhs.fields.get(tuple_index) {
                rhs = child.as_ref();
            } else if let Some(dependency) = &rhs.dependency {
                // RHS has a broader dependency, extend it with the remaining index
                let mut specific_dependency = dependency.clone();
                specific_dependency.extend_from_slice(rhs_index);
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

    pub fn set_dependency(
        &mut self,
        index: &StructOrTupleIndex,
        input_tuple_index: StructOrTupleIndex,
    ) {
        let child = self.create_child(index.clone());
        child.dependency = Some(input_tuple_index);
    }
}

impl PartialEq for StructOrTuple {
    fn eq(&self, other: &Self) -> bool {
        self.dependency == other.dependency && self.fields == other.fields
    }
}

// Find whether a tuple's usage (Ex: a.0.1) references an existing var (Ex: a), and if so, calculate the new StructOrTupleIndex
#[derive(Default)]
struct StructOrTupleUseRhs {
    existing_dependencies: HashMap<syn::Ident, StructOrTuple>,
    rhs_tuple: StructOrTuple,
    field_index: StructOrTupleIndex, /* Used to track where we are in the tuple/struct as we recurse. Ex: ((a, b), c) -> [0, 1] for b */
    reference_field_index: StructOrTupleIndex, /* Used to track the index of the tuple/struct that we're referencing. Ex: a.0.1 -> [0, 1] */
}

impl Visit<'_> for StructOrTupleUseRhs {
    fn visit_expr_path(&mut self, path: &syn::ExprPath) {
        if let Some(ident) = path.path.get_ident() {
            // Base path matches an Ident that has an existing dependency in one of its fields
            if let Some(existing_dependency) = self.existing_dependencies.get(ident) {
                self.rhs_tuple.set_dependencies(
                    &self.field_index,
                    existing_dependency,
                    &self.reference_field_index,
                );
            }
        }
    }

    fn visit_expr_field(&mut self, expr: &syn::ExprField) {
        // Find the ident of the rightmost field
        let field = match &expr.member {
            syn::Member::Named(ident) => ident.to_string(),
            syn::Member::Unnamed(index) => index.index.to_string(),
        };

        // Keep going left until we get to the root
        self.reference_field_index.insert(0, field);
        self.visit_expr(expr.base.as_ref());
    }

    fn visit_expr_tuple(&mut self, tuple: &syn::ExprTuple) {
        // Recursively visit elems, in case we have nested tuples
        let pre_recursion_index = self.field_index.clone();
        for (i, elem) in tuple.elems.iter().enumerate() {
            self.field_index = pre_recursion_index.clone();
            self.field_index.push(i.to_string());
            // Reset field index
            self.reference_field_index.clear();
            self.visit_expr(elem);
        }
    }

    fn visit_expr_struct(&mut self, struc: &syn::ExprStruct) {
        let pre_recursion_index = self.field_index.clone();
        for field in &struc.fields {
            self.field_index = pre_recursion_index.clone();
            let field_name = match &field.member {
                syn::Member::Named(ident) => ident.to_string(),
                syn::Member::Unnamed(_) => {
                    panic!("Struct cannot have unnamed field: {:?}", struc);
                }
            };
            self.field_index.push(field_name);
            // Reset field index
            self.reference_field_index.clear();
            self.visit_expr(&field.expr);
        }

        // For structs of the form struct { a: 1, ..rest }
        if struc.rest.is_some() {
            panic!(
                "Partitioning analysis does not support structs with rest fields: {:?}",
                struc
            );
        }
    }
}

// Create a mapping from Ident to tuple indices (Note: Not necessarily input tuple indices)
// For example, (a, (b, c)) -> { a: [0], b: [1, 0], c: [1, 1] }
#[derive(Default)]
struct TupleDeclareLhs {
    lhs_tuple: HashMap<syn::Ident, StructOrTupleIndex>,
    tuple_index: StructOrTupleIndex, // Internal, used to track the index of the tuple recursively
}

impl TupleDeclareLhs {
    fn into_tuples(&self) -> HashMap<syn::Ident, StructOrTuple> {
        let mut tuples = HashMap::new();
        for (ident, index) in &self.lhs_tuple {
            let mut tuple = StructOrTuple::default();
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
                    self.tuple_index.push(i.to_string());
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
    output_dependencies: StructOrTuple,
    dependencies: HashMap<syn::Ident, StructOrTuple>,
}

impl Visit<'_> for EqualityAnalysis {
    fn visit_expr_return(&mut self, _: &syn::ExprReturn) {
        panic!("Partitioning analysis does not support return.");
    }

    fn visit_stmt(&mut self, stmt: &syn::Stmt) {
        match stmt {
            syn::Stmt::Local(local) => {
                // Analyze LHS
                let mut input_analysis: TupleDeclareLhs = TupleDeclareLhs::default();
                input_analysis.visit_pat(&local.pat);

                // Analyze RHS
                let mut analysis = StructOrTupleUseRhs::default();
                if let Some(init) = local.init.as_ref() {
                    // See if RHS is a direct match for an existing dependency
                    analysis.existing_dependencies = self.dependencies.clone();
                    analysis.visit_expr(init.expr.as_ref());
                }

                // Set dependencies from LHS to RHS
                for (lhs, tuple_index) in input_analysis.lhs_tuple.iter() {
                    let mut tuple = StructOrTuple::default();
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
                        let mut analysis = StructOrTupleUseRhs::default();
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
    pub output_dependencies: StructOrTuple,
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
    pub output_dependencies: HashMap<usize, StructOrTuple>, /* Map from stmt_id to output tuple dependencies */
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

    use crate::partition_analysis::{StructOrTuple, partition_analysis};

    fn verify_abcde_tuple(builder: FlowBuilder<'_>) {
        let built = builder.with_default_optimize::<DeployRuntime>();
        let mut ir = deep_clone(built.ir());
        let metadata = partition_analysis(&mut ir);

        let mut expected_output_dependency = StructOrTuple::default();
        expected_output_dependency.set_dependency(&vec!["0".to_string()], vec!["0".to_string()]);
        expected_output_dependency.set_dependency(&vec!["1".to_string()], vec!["1".to_string()]);
        expected_output_dependency.set_dependency(
            &vec!["2".to_string(), "0".to_string()],
            vec!["2".to_string(), "0".to_string()],
        );
        expected_output_dependency.set_dependency(
            &vec!["2".to_string(), "1".to_string(), "0".to_string()],
            vec!["2".to_string(), "1".to_string(), "0".to_string()],
        );
        expected_output_dependency.set_dependency(&vec!["3".to_string()], vec!["3".to_string()]);
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

    #[test]
    fn test_tuple_input_output_implicit_nesting() {
        let builder = FlowBuilder::new();
        let cluster = builder.cluster::<()>();
        cluster
            .source_iter(q!([(1, 2, (3, (4,)), 5)]))
            .map(q!(|(a, b, cd, e)| {
                let f = cd;
                (a, b, (f.0, (f.1.0,)), e)
            }))
            .for_each(q!(|(a, b, (c, (d,)), e)| {
                println!("a: {}, b: {}, c: {}, d: {}, e: {}", a, b, c, d, e);
            }));
        verify_abcde_tuple(builder);
    }

    #[test]
    fn test_tuple_combined() {
        let builder = FlowBuilder::new();
        let cluster = builder.cluster::<()>();
        cluster
            .source_iter(q!([(1, 2, (3, (4,)), 5)]))
            .map(q!(|(a, b, (c, (d,)), e)| {
                let f = (d,);
                let g = (c, f);
                let h = (a, b, g, e);
                h
            }))
            .for_each(q!(|(a, b, (c, (d,)), e)| {
                println!("a: {}, b: {}, c: {}, d: {}, e: {}", a, b, c, d, e);
            }));
        verify_abcde_tuple(builder);
    }

    struct TestStruct {
        a: usize,
        b: String,
        c: Option<usize>,
    }

    struct TestNestedStruct {
        struct_1: TestStruct,
        struct_2: TestStruct,
    }

    fn verify_struct(builder: FlowBuilder<'_>) {
        let built = builder.with_default_optimize::<DeployRuntime>();
        let mut ir = deep_clone(built.ir());
        let metadata = partition_analysis(&mut ir);

        let mut expected_output_dependency = StructOrTuple::default();
        expected_output_dependency.set_dependency(
            &vec!["struct_1".to_string(), "a".to_string()],
            vec!["a".to_string()],
        );
        expected_output_dependency.set_dependency(
            &vec!["struct_1".to_string(), "b".to_string()],
            vec!["b".to_string()],
        );
        expected_output_dependency.set_dependency(
            &vec!["struct_1".to_string(), "c".to_string()],
            vec!["c".to_string()],
        );
        expected_output_dependency.set_dependency(
            &vec!["struct_2".to_string(), "a".to_string()],
            vec!["a".to_string()],
        );
        expected_output_dependency.set_dependency(
            &vec!["struct_2".to_string(), "b".to_string()],
            vec!["b".to_string()],
        );
        expected_output_dependency.set_dependency(
            &vec!["struct_2".to_string(), "c".to_string()],
            vec!["c".to_string()],
        );
        assert_eq!(
            metadata.output_dependencies.get(&1),
            Some(&expected_output_dependency)
        );

        let _ = built.compile(&RuntimeData::new("FAKE"));
    }

    #[test]
    fn test_nested_struct() {
        let builder = FlowBuilder::new();
        let cluster = builder.cluster::<()>();
        cluster
            .source_iter(q!([TestStruct {
                a: 1,
                b: "test".to_string(),
                c: Some(3),
            }]))
            .map(q!(|test_struct| {
                let struct1 = TestStruct {
                    a: test_struct.a,
                    b: test_struct.b,
                    c: test_struct.c,
                };
                let struct2 = TestStruct {
                    a: struct1.a,
                    b: struct1.b.clone(),
                    c: struct1.c,
                };
                TestNestedStruct {
                    struct_1: struct1,
                    struct_2: struct2,
                }
            }))
            .for_each(q!(|_nested_struct| {
                println!("Done");
            }));
        verify_struct(builder);
    }

    #[test]
    fn test_nested_struct_declaration() {
        let builder = FlowBuilder::new();
        let cluster = builder.cluster::<()>();
        cluster
            .source_iter(q!([TestStruct {
                a: 1,
                b: "test".to_string(),
                c: Some(3),
            }]))
            .map(q!(|test_struct| {
                TestNestedStruct {
                    struct_1: TestStruct {
                        a: test_struct.a,
                        b: test_struct.b.clone(),
                        c: test_struct.c,
                    },
                    struct_2: TestStruct {
                        a: test_struct.a,
                        b: test_struct.b,
                        c: test_struct.c,
                    },
                }
            }))
            .for_each(q!(|_nested_struct| {
                println!("Done");
            }));
        verify_struct(builder);
    }

    #[test]
    fn test_struct_implicit_field() {
        let builder = FlowBuilder::new();
        let cluster = builder.cluster::<()>();
        cluster
            .source_iter(q!([TestStruct {
                a: 1,
                b: "test".to_string(),
                c: Some(3),
            }]))
            .map(q!(|test_struct| {
                let struct_1 = TestStruct {
                    a: test_struct.a,
                    b: test_struct.b.clone(),
                    c: test_struct.c,
                };
                TestNestedStruct {
                    struct_1,
                    struct_2: TestStruct {
                        a: test_struct.a,
                        b: test_struct.b,
                        c: test_struct.c,
                    },
                }
            }))
            .for_each(q!(|_nested_struct| {
                println!("Done");
            }));
        verify_struct(builder);
    }
}
