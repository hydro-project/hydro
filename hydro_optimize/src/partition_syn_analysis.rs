use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use syn::visit::Visit;

pub type StructOrTupleIndex = Vec<String>; // Ex: ["a", "b"] represents x.a.b

// Invariant: Cannot have both a dependency and fields (fields are more specific)
#[derive(Debug, Clone, Default, Eq)]
pub struct StructOrTuple {
    dependency: Option<StructOrTupleIndex>, // Input tuple index this tuple is equal to, if any
    fields: HashMap<String, Box<StructOrTuple>>, // Fields 1 layer deep
}

impl StructOrTuple {
    pub fn new_completely_dependent() -> Self {
        StructOrTuple {
            dependency: Some(vec![]), /* Empty dependency means it is completely dependent on the input tuple */
            fields: HashMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
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

    pub fn get_dependency(&self, index: &StructOrTupleIndex) -> Option<StructOrTuple> {
        let mut child = self;
        for (i, field) in index.iter().enumerate() {
            if let Some(grandchild) = child.fields.get(field) {
                child = grandchild.as_ref();
            } else if let Some(dependency) = &child.dependency {
                // If the dependency is broader, create a specific child
                let mut specific_dependency = dependency.clone();
                specific_dependency.extend_from_slice(&index[i..]);
                let mut new_child = StructOrTuple::default();
                new_child.dependency = Some(specific_dependency.clone());
                return Some(new_child);
            } else {
                return None; // No dependency or child
            }
        }
        Some(child.clone())
    }

    fn intersect_children(
        tuple1: &StructOrTuple,
        tuple_with_fields: &StructOrTuple,
    ) -> StructOrTuple {
        let mut new_tuple = StructOrTuple::default();
        for (field, tuple2_child) in &tuple_with_fields.fields {
            // Construct a child for tuple1 if it has a broader dependency
            let tuple1_child = tuple1
                .fields
                .get(field)
                .and_then(|boxed| Some((**boxed).clone()))
                .or_else(|| {
                    if let Some(dependency1) = &tuple1.dependency {
                        let mut child_dependency = dependency1.clone();
                        child_dependency.push(field.clone());
                        Some(StructOrTuple {
                            dependency: Some(child_dependency),
                            fields: HashMap::new(),
                        })
                    } else {
                        None
                    }
                });
            // Recursively check if there's a match in the child
            if let Some(tuple1_child) = tuple1_child {
                if let Some(shared_child) = StructOrTuple::intersect(&tuple1_child, &tuple2_child) {
                    new_tuple
                        .fields
                        .insert(field.clone(), Box::new(shared_child));
                }
            }
        }
        new_tuple
    }

    // Create a tuple representing dependencies present in both tuples, keeping the more specific dependency if there is one
    pub fn intersect(tuple1: &StructOrTuple, tuple2: &StructOrTuple) -> Option<StructOrTuple> {
        let new_tuple = if let Some(dependency1) = &tuple1.dependency {
            if let Some(dependency2) = &tuple2.dependency {
                // Exact shared dependency, return
                if dependency1 == dependency2 {
                    return Some(tuple1.clone());
                } else {
                    return None;
                }
            } else {
                // tuple2 has no dependency, check its fields
                StructOrTuple::intersect_children(tuple1, tuple2)
            }
        } else {
            // tuple1 has no dependency, check its fields
            StructOrTuple::intersect_children(tuple2, tuple1)
        };

        if new_tuple.is_empty() {
            None
        } else {
            Some(new_tuple)
        }
    }

    pub fn intersect_tuples(tuples: &[StructOrTuple]) -> Option<StructOrTuple> {
        if tuples.is_empty() {
            return None;
        }

        let mut intersection = tuples[0].clone();
        for tuple in &tuples[1..] {
            if let Some(shared) = StructOrTuple::intersect(&intersection, tuple) {
                intersection = shared;
            } else {
                return None; // No shared dependencies
            }
        }
        Some(intersection)
    }

    /// Remap dependencies of the parent onto the child
    ///
    /// The parent's dependencies are absolute (dependency on an input to the node);
    /// the child's dependencies are relative (dependency within the function).
    pub fn project_parent(parent: &StructOrTuple, child: &StructOrTuple) -> Option<StructOrTuple> {
        // Child depends on a field of the parent
        if let Some(dependency) = &child.dependency {
            // For that field, the parent depends on a field of the input
            if let Some(dependency_in_parent) = parent.get_dependency(&dependency) {
                // Track the input field
                Some(dependency_in_parent.clone())
            } else {
                None
            }
        } else {
            // Recurse
            let mut new_child = StructOrTuple::default();
            for (field, child_field) in &child.fields {
                if let Some(field_with_counterpart_in_parent) =
                    StructOrTuple::project_parent(parent, child_field)
                {
                    new_child
                        .fields
                        .insert(field.clone(), Box::new(field_with_counterpart_in_parent));
                }
            }
            if new_child.is_empty() {
                None
            } else {
                Some(new_child)
            }
        }
    }
}

impl PartialEq for StructOrTuple {
    fn eq(&self, other: &Self) -> bool {
        self.dependency == other.dependency && self.fields == other.fields
    }
}

impl Hash for StructOrTuple {
    fn hash<H: Hasher>(&self, state: &mut H) {
        format!("{:#?}", self).hash(state);
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

impl StructOrTupleUseRhs {
    fn add_to_rhs_tuple(&mut self, dependency: &StructOrTuple) {
        if !dependency.is_empty() {
            self.rhs_tuple.set_dependencies(
                &self.field_index,
                dependency,
                &self.reference_field_index,
            );
        }
    }
}

impl Visit<'_> for StructOrTupleUseRhs {
    fn visit_expr_path(&mut self, path: &syn::ExprPath) {
        if let Some(ident) = path.path.get_ident() {
            // Base path matches an Ident that has an existing dependency in one of its fields
            if let Some(existing_dependency) = self.existing_dependencies.get(ident).cloned() {
                self.add_to_rhs_tuple(&existing_dependency);
            }
            else if ident.to_string() == "None" {
                println!("Warning: Found keyword 'None', which will be ignored by partitioning analysis.")
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

    fn visit_expr_method_call(&mut self, call: &syn::ExprMethodCall) {
        if call.method.to_string() == "clone" {
            // Allow "clone", since it doesn't change the RHS
            self.visit_expr(&call.receiver);
        } else {
            println!(
                "StructOrTupleUseRhs skipping unsupported RHS method call: {:?}",
                call
            );
        }
    }

    fn visit_expr_block(&mut self, block: &syn::ExprBlock) {
        // Analyze the block, copying over our existing dependencies
        let mut block_analysis = EqualityAnalysis::default();
        block_analysis.dependencies = self.existing_dependencies.clone();
        block_analysis.visit_expr_block(block);
        // If there is an output, and there is a dependency, set it
        if !block_analysis.output_dependencies.is_empty() {
            self.add_to_rhs_tuple(&block_analysis.output_dependencies);
        }
    }

    fn visit_expr_if(&mut self, expr: &syn::ExprIf) {
        // Don't consider if else branch doesn't exist, since the return value will just be ()
        // Note: The if condition is irrelevant so it's not analyzed
        let mut branch_dependencies = vec![];
        let mut if_expr = expr;

        // Since we may have multiple else-ifs, keep unwrapping the else branch until we reach a block
        loop {
            if let Some(else_branch) = &if_expr.else_branch {
                let mut then_branch_analysis = EqualityAnalysis::default();
                then_branch_analysis.dependencies = self.existing_dependencies.clone();
                then_branch_analysis.visit_block(&if_expr.then_branch);
                branch_dependencies.push(then_branch_analysis.output_dependencies.clone());

                match &*else_branch.1 {
                    syn::Expr::Block(block) => {
                        let mut else_branch_analysis = EqualityAnalysis::default();
                        else_branch_analysis.dependencies = self.existing_dependencies.clone();
                        else_branch_analysis.visit_expr_block(block);
                        branch_dependencies.push(else_branch_analysis.output_dependencies.clone());
                        break;
                    }
                    syn::Expr::If(nested_if_expr) => {
                        if_expr = nested_if_expr;
                    }
                    _ => panic!("Unexpected else branch expression: {:?}", else_branch.1),
                }
            } else {
                // Do not process the if statement if there is a missing else branch, the return type will be ()
                return;
            }
        }

        // Set the dependency to whatever is shared between the outputs of all branches
        if let Some(shared) = StructOrTuple::intersect_tuples(&branch_dependencies) {
            self.add_to_rhs_tuple(&shared);
        }
    }

    fn visit_expr_match(&mut self, expr: &syn::ExprMatch) {
        let mut branch_dependencies = vec![];
        for arm in &expr.arms {
            let mut arm_analysis = EqualityAnalysis::default();
            arm_analysis.dependencies = self.existing_dependencies.clone();
            arm_analysis.visit_expr(&arm.body);

            if arm_analysis.output_dependencies.is_empty() {
                return; // One arm is empty, no dependencies
            }
            branch_dependencies.push(arm_analysis.output_dependencies.clone());
        }

        if let Some(shared) = StructOrTuple::intersect_tuples(&branch_dependencies) {
            self.add_to_rhs_tuple(&shared);
        }
    }

    fn visit_expr_call(&mut self, call: &syn::ExprCall) {
        // Allow "Some" keyword (for Options)
        if let syn::Expr::Path(func) = call.func.as_ref() {
            if func.path.is_ident("Some") {
                self.visit_expr(&call.args[0]); // Visit the argument of Some
            }
        }
    }

    fn visit_expr(&mut self, expr: &syn::Expr) {
        match expr {
            syn::Expr::Path(path) => self.visit_expr_path(path),
            syn::Expr::Field(field) => self.visit_expr_field(field),
            syn::Expr::Tuple(tuple) => self.visit_expr_tuple(tuple),
            syn::Expr::Struct(struc) => self.visit_expr_struct(struc),
            syn::Expr::MethodCall(call) => self.visit_expr_method_call(call),
            syn::Expr::Cast(cast) => self.visit_expr(&cast.expr), /* Allow casts assuming they don't truncate the RHS */
            syn::Expr::Block(block) => self.visit_expr_block(block),
            syn::Expr::If(if_expr) => self.visit_expr_if(if_expr),
            syn::Expr::Match(match_expr) => self.visit_expr_match(match_expr),
            syn::Expr::Call(call_expr) => self.visit_expr_call(call_expr),
            _ => println!(
                "StructOrTupleUseRhs skipping unsupported RHS expression: {:?}",
                expr
            ),
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
                println!(
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

    fn visit_block(&mut self, block: &syn::Block) {
        for (i, stmt) in block.stmts.iter().enumerate() {
            self.visit_stmt(stmt);

            if i == block.stmts.len() - 1 {
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

    fn visit_expr(&mut self, expr: &syn::Expr) {
        match expr {
            syn::Expr::Return(_) => panic!("Partitioning analysis does not support return."),
            syn::Expr::Block(block) => self.visit_expr_block(block),
            _ => {
                // Visit other expressions to analyze dependencies
                let mut analysis = StructOrTupleUseRhs::default();
                analysis.existing_dependencies = self.dependencies.clone();
                analysis.visit_expr(expr);
                self.output_dependencies = analysis.rhs_tuple;
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

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;

    use hydro_lang::deploy::DeployRuntime;
    use hydro_lang::ir::{HydroLeaf, HydroNode, deep_clone, traverse_dfir};
    use hydro_lang::{FlowBuilder, Location};
    use stageleft::{RuntimeData, q};
    use syn::visit::Visit;

    use crate::partition_syn_analysis::{AnalyzeClosure, StructOrTuple};

    fn partition_analysis_leaf(
        leaf: &mut HydroLeaf,
        next_stmt_id: &mut usize,
        metadata: &RefCell<HashMap<usize, StructOrTuple>>,
    ) {
        let mut analyzer = AnalyzeClosure::default();
        leaf.visit_debug_expr(|debug_expr| {
            analyzer.visit_expr(&debug_expr.0);
        });
        metadata
            .borrow_mut()
            .insert(*next_stmt_id, analyzer.output_dependencies.clone());
    }

    fn partition_analysis_node(
        node: &mut HydroNode,
        next_stmt_id: &mut usize,
        metadata: &RefCell<HashMap<usize, StructOrTuple>>,
    ) {
        let mut analyzer = AnalyzeClosure::default();
        node.visit_debug_expr(|debug_expr| {
            analyzer.visit_expr(&debug_expr.0);
        });
        metadata
            .borrow_mut()
            .insert(*next_stmt_id, analyzer.output_dependencies.clone());
    }

    fn partition_analysis(ir: &mut [HydroLeaf]) -> HashMap<usize, StructOrTuple> {
        let partitioning_metadata = RefCell::new(HashMap::new());
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

    fn verify_tuple(builder: FlowBuilder<'_>, expected_output_dependency: &StructOrTuple) {
        let built = builder.with_default_optimize::<DeployRuntime>();
        let mut ir = deep_clone(built.ir());
        let actual_dependencies = partition_analysis(&mut ir);

        assert_eq!(
            actual_dependencies.get(&1),
            Some(expected_output_dependency)
        );

        let _ = built.compile(&RuntimeData::new("FAKE"));
    }

    fn verify_abcde_tuple(builder: FlowBuilder<'_>) {
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

        verify_tuple(builder, &expected_output_dependency);
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
    fn test_tuple_no_block() {
        let builder = FlowBuilder::new();
        let cluster = builder.cluster::<()>();
        cluster
            .source_iter(q!([(1, 2, (3, (4,)), 5)]))
            .map(q!(|(a, b, cd, e)| (a, b, (cd.0, (cd.1.0,)), e)))
            .for_each(q!(|(a, b, (c, (d,)), e)| {
                println!("a: {}, b: {}, c: {}, d: {}, e: {}", a, b, c, d, e);
            }));
        verify_abcde_tuple(builder);
    }

    #[test]
    fn test_if_shared_intersection() {
        let builder = FlowBuilder::new();
        let cluster = builder.cluster::<()>();
        cluster
            .source_iter(q!([(1, 2, (3, (4,)), 5)]))
            .map(q!(|(a, b, (c, (d,)), e)| {
                let f = (d,);
                let g = (c, f);
                (a, b, if f == (4,) { g } else { (c, (d,)) }, e)
            }))
            .for_each(q!(|(a, b, (c, (d,)), e)| {
                println!("a: {}, b: {}, c: {}, d: {}, e: {}", a, b, c, d, e);
            }));
        verify_abcde_tuple(builder);
    }

    #[test]
    fn test_if_conflicting_intersection() {
        let builder = FlowBuilder::new();
        let cluster = builder.cluster::<()>();
        cluster
            .source_iter(q!([(1, 2, (3, (4,)), 5)]))
            .map(q!(|(a, b, (c, (d,)), e)| {
                let f = (d,);
                (a, b, if f == (4,) { (c, (d, b)) } else { (c, (d, e)) }, e)
            }))
            .for_each(q!(|(a, b, (c, (d, _x)), e)| {
                println!("a: {}, b: {}, c: {}, d: {}, e: {}", a, b, c, d, e);
            }));
        verify_abcde_tuple(builder);
    }

    #[test]
    fn test_if_implicit_expansion() {
        let builder = FlowBuilder::new();
        let cluster = builder.cluster::<()>();
        cluster
            .source_iter(q!([(1, 2, (3, (4,)), 5)]))
            .map(q!(|(a, b, cd, e)| {
                (a, b, if a == 1 { cd } else { (cd.0, (cd.1.0,)) }, e)
            }))
            .for_each(q!(|(a, b, (c, (d,)), e)| {
                println!("a: {}, b: {}, c: {}, d: {}, e: {}", a, b, c, d, e);
            }));
        verify_abcde_tuple(builder);
    }

    #[test]
    fn test_else_if() {
        let builder = FlowBuilder::new();
        let cluster = builder.cluster::<()>();
        cluster
            .source_iter(q!([(1, 2, (3, (4,)), 5)]))
            .map(q!(|(a, b, (c, (d,)), e)| {
                let f = (d,);
                let g = (c, f);
                (
                    a,
                    b,
                    if f == (4,) {
                        g
                    } else if f == (3,) {
                        (c, f)
                    } else {
                        (c, (d,))
                    },
                    e,
                )
            }))
            .for_each(q!(|(a, b, (c, (d,)), e)| {
                println!("a: {}, b: {}, c: {}, d: {}, e: {}", a, b, c, d, e);
            }));
        verify_abcde_tuple(builder);
    }

    #[test]
    fn test_if_option() {
        let builder = FlowBuilder::new();
        let cluster = builder.cluster::<()>();
        cluster
            .source_iter(q!([(1, 2, (3, (4,)), 5)]))
            .map(q!(|(a, b, (c, (d,)), e)| Some((a, b, (c, (d,)), e))))
            .for_each(q!(|x| {
                println!("x: {:?}", x);
            }));
        verify_abcde_tuple(builder);
    }

    #[test]
    fn test_match() {
        let builder = FlowBuilder::new();
        let cluster = builder.cluster::<()>();
        cluster
            .source_iter(q!([(1, 2, (3, (4,)), 5)]))
            .map(q!(|(a, b, (c, (d,)), e)| {
                let f = (d,);
                let g = (c, f);
                let cd = match f {
                    (4,) => g,
                    (3,) => (c, f),
                    _ => (c, (d,)),
                };
                (a, b, cd, e)
            }))
            .for_each(q!(|(a, b, (c, (d,)), e)| {
                println!("a: {}, b: {}, c: {}, d: {}, e: {}", a, b, c, d, e);
            }));
        verify_abcde_tuple(builder);
    }

    #[test]
    fn test_block() {
        let builder = FlowBuilder::new();
        let cluster = builder.cluster::<()>();
        cluster
            .source_iter(q!([(1, 2, (3, (4,)), 5)]))
            .map(q!(|(a, b, (c, (d,)), e)| {
                let cd = {
                    let f = (d,);
                    let g = (c, f);
                    g
                };
                (a, b, cd, e)
            }))
            .for_each(q!(|(a, b, (c, (d,)), e)| {
                println!("a: {}, b: {}, c: {}, d: {}, e: {}", a, b, c, d, e);
            }));
        verify_abcde_tuple(builder);
    }

    #[test]
    fn test_nested_block() {
        let builder = FlowBuilder::new();
        let cluster = builder.cluster::<()>();
        cluster
            .source_iter(q!([(1, 2, (3, (4,)), 5)]))
            .map(q!(|(a, b, (c, (d,)), e)| {
                let cd = {
                    let f = (d,);
                    let g = {
                        let h = (c, f);
                        h
                    };
                    g
                };
                (a, b, cd, e)
            }))
            .for_each(q!(|(a, b, (c, (d,)), e)| {
                println!("a: {}, b: {}, c: {}, d: {}, e: {}", a, b, c, d, e);
            }));
        verify_abcde_tuple(builder);
    }

    #[test]
    fn test_block_shadowing() {
        let builder = FlowBuilder::new();
        let cluster = builder.cluster::<()>();
        cluster
            .source_iter(q!([(1, 2, (3, (4,)), 5)]))
            .map(q!(|(a, b, (c, (d,)), e)| {
                let cd = {
                    let f = (d,);
                    let b = {
                        let a = (c, f);
                        a
                    };
                    b
                };
                (a, b, cd, e)
            }))
            .for_each(q!(|(a, b, (c, (d,)), e)| {
                println!("a: {}, b: {}, c: {}, d: {}, e: {}", a, b, c, d, e);
            }));
        verify_abcde_tuple(builder);
    }

    #[test]
    fn test_full_assignment() {
        let builder = FlowBuilder::new();
        let cluster = builder.cluster::<()>();
        cluster
            .source_iter(q!([(1, 2, (3, (4,)), 5)]))
            .map(q!(|a| { a }))
            .for_each(q!(|(a, b, (c, (d,)), e)| {
                println!("a: {}, b: {}, c: {}, d: {}, e: {}", a, b, c, d, e);
            }));

        let expected_output_dependency = StructOrTuple::new_completely_dependent();
        verify_tuple(builder, &expected_output_dependency);
    }

    #[derive(Clone)]
    struct TestStruct {
        a: usize,
        b: String,
        c: Option<usize>,
    }

    #[allow(dead_code)]
    struct TestNestedStruct {
        struct_1: TestStruct,
        struct_2: TestStruct,
    }

    fn verify_struct(builder: FlowBuilder<'_>) {
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
        verify_tuple(builder, &expected_output_dependency);
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
                let struct_1 = test_struct.clone();
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

        let mut expected_output_dependency = StructOrTuple::default();
        expected_output_dependency.set_dependency(&vec!["struct_1".to_string()], vec![]);
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

        verify_tuple(builder, &expected_output_dependency);
    }
}
