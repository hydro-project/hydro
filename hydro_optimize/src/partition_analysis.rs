use std::{collections::{HashMap, HashSet}, path};

use syn::visit::Visit;

type TupleIndex = Vec<usize>; // Ex: [0,1] represents a.0.1

#[derive(Debug, Clone, Default)]
struct Tuple {
    dependency: Option<TupleIndex>, // Input tuple index this tuple is equal to, if any
    fields: HashMap<usize, Box<Tuple>>, // Fields 1 layer deep
}

impl Tuple {
    fn is_empty(&self) -> bool {
        self.dependency.is_none() && self.fields.is_empty()
    }

    fn set_dependencies(&mut self, dependencies: &Tuple) {
        if let Some(dependency) = &dependencies.dependency {
            self.dependency = Some(dependency.clone());
        }
        else {
            // Recursively copy dependencies, creating new child tuples as needed
            for (field_index, field) in dependencies.fields.iter() {
                let tuple_child = self.fields.entry(*field_index)
                    .or_insert_with(|| Box::new(Tuple::default()));
                tuple_child.set_dependencies(field);
            }
        }
    }

    fn set_dependency(&mut self, index: &TupleIndex, input_tuple_index: TupleIndex) {
        if index.is_empty() {
            // If the index is empty, this is the root tuple
            self.dependency = Some(input_tuple_index);
        } else {
            // Otherwise, insert into the fields recursively, creating as needed
            let field_index = index[0];
            let remaining_index = index[1..].to_vec();
            let tuple_child = self.fields.entry(field_index)
                .or_insert_with(|| Box::new(Tuple::default()));
            tuple_child.set_dependency(&remaining_index, input_tuple_index);
        }
    }

    fn get_dependency(&self, index: &TupleIndex) -> Option<TupleIndex> {
        if index.is_empty() {
            // If the index is empty, this is the root tuple
            self.dependency
        } else {
            // Otherwise, get the field recursively
            let field_index = index[0];
            let remaining_index = &index[1..];
            self.fields.get(&field_index)
                .and_then(|child| child.get_dependency(remaining_index))
        }
    }
}

// Find whether a tuple's usage (Ex: a.0.1) references an existing var (Ex: a), and if so, calculate the new TupleIndex
#[derive(Default)]
struct TupleUseRhs {
    existing_dependencies: HashMap<syn::Ident, Tuple>,
    rhs_tuple: Tuple,
    tuple_position_in_paren: TupleIndex, // Used to track where we are in the tuple as we recurse. Ex: ((a, b), c) -> [0, 1] for b
    tuple_field_index: TupleIndex, // Used to track the index of the tuple recursively. Ex: a.0.1 -> [0, 1]
}

impl Visit<'_> for TupleUseRhs {
    fn visit_expr_path(&mut self, path: &syn::ExprPath) {
        if let Some(ident) = path.path.get_ident() {
            // Base path matches an Ident that has an existing dependency in one of its fields
            if let Some(existing_dependency) = self.existing_dependencies.get(ident) {
                // Check if the relevant field actually has a dependency
                if let Some(input_dependency) = existing_dependency.get_dependency(&self.tuple_field_index) {
                    self.rhs_tuple.set_dependency(
                        &self.tuple_position_in_paren,
                        input_dependency.clone(),
                    );
                }
            }
        }
    }

    fn visit_expr_field(&mut self, expr: &syn::ExprField) {
        // Find the ident of the rightmost field
        let index = match expr.member {
            syn::Member::Named(ident) => {
                panic!("Partitioning analysis currently supports only tuples, not structs. Found a named field on a struct: {:?}", ident);
            },
            syn::Member::Unnamed(index) => {
                index.index as usize
            }
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
    lhs_tuple: HashMap<syn::Ident, TupleIndex>, // Ident -> TupleIndex
    tuple_index: TupleIndex, // Internal, used to track the index of the tuple recursively
}

impl Visit<'_> for TupleDeclareLhs {
    fn visit_pat(&mut self, pat: &syn::Pat) {
        match pat {
            syn::Pat::Ident(ident) => {
                self.lhs_tuple.insert(ident.ident.clone(), self.tuple_index);
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
                panic!("TupleDeclareLhs does not support this LHS pattern: {:?}", pat);
            }
        }
    }
}

#[derive(Default)]
struct EqualityAnalysis {
    output_dependencies: Tuple,
    dependencies: HashMap<syn::Ident, Tuple>,
}

impl EqualityAnalysis {
    // Creates a dependency for the LHS Ident (and any of its fields) where the RHS has a dependency
    // Do a deep search of the RHS after matching as much of its prefix as possible to the LHS
    fn assign_tuple(&self, lhs: &TupleIndex, mut rhs: &Tuple) -> Option<Tuple> {
        let mut tuple = Tuple::default();

        // Case 1: LHS = Ident
        if lhs.is_empty() {
            if !rhs.is_empty() {
                tuple.set_dependencies(rhs);
                return Some(tuple);
            }
            else {
                return None;
            }
        }

        // Case 2: LHS is a tuple with fields
        for (i, position) in lhs.iter().enumerate() {
            if let Some(field) = rhs.fields.get(position) {
                // Case 2.1: RHS contains the same field as the LHS, continue searching in RHS
                rhs = field.as_ref();
            }
            else if let Some(dependency) = rhs.dependency {
                // Case 2.2: RHS contains a broader dependency. Ex: let (a, b) = c, where c depends on d in the input.
                let remaining_lhs = &lhs[i..];
                let mut specific_rhs_dependency = dependency.clone();
                specific_rhs_dependency.extend(remaining_lhs);
                tuple.dependency = Some(specific_rhs_dependency);
                return Some(tuple);
            }
            else {
                // Case 2.3: RHS doesn't contain dependencies for this LHS
                return None;
            }
        }

        // Case 2.1 continued: All LHS fields matched, RHS might have more specific dependencies. Ex: let (a, b) = ((c, d), e). 
        tuple.set_dependencies(rhs);
        Some(tuple)
    }
}

impl Visit<'_> for EqualityAnalysis {
    fn visit_expr(&mut self, expr: &syn::Expr) {
        // Filter Expr types that we do not support
        match expr {
            syn::Expr::Return(_) => {
                panic!("Partitioning analysis does not support: {:?}.", expr);
            }
            _ => {
                self.visit_expr(expr);
            }
        }
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
                    analysis.visit_expr(init);
                }

                // Set dependencies from LHS to RHS
                for (lhs, tuple_index) in input_analysis.indices.iter() {
                    let tuple = self.assign_tuple(tuple_index, &analysis.rhs_tuple);
                    if let Some(tuple) = tuple {
                        // Found a match, insert into dependencies
                        println!("Found dependency: {} {:?} = {:?}", lhs, tuple_index, tuple);
                        self.dependencies.insert(lhs.clone(), tuple);
                    } else {
                        // No RHS dependency found, delete LHS if it exists (it shadows any previous dependency)
                        self.dependencies.remove(lhs);
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
                    if let Some(_) = semicolon {
                        // No semicolon, no output
                        self.output_dependencies.clear();
                    }
                    else {
                        let mut analysis = TupleUseRhs::default();
                        analysis.existing_dependencies = self.dependencies.clone();
                        analysis.visit_expr(expr);

                        self.output_dependencies = analysis.rhs_tuple;
                        println!("Output dependency: {:?}", analysis.rhs_tuple);
                    }
                }
            }
        }
    }
}

#[derive(Default)]
pub struct AnalyzeClosure {
    found_closure: bool, // Used to avoid executing visit_pat on anything but the function body
    analysis_results: EqualityAnalysis,
}

impl Visit<'_> for AnalyzeClosure {
    fn visit_expr_closure(&mut self, closure: &syn::ExprClosure) {
        if self.found_closure {
            panic!("Multiple top-level closures found in a single Expr during partitioning analysis, likely due to running analysis over a function such as reduce.");
        }

        // Find all input vars
        self.found_closure = true;
        if closure.inputs.len() > 1 {
            panic!("Partitioning analysis does not currently support closures with multiple inputs (such as reduce): {:?}.", closure);
        }
        let mut input_analysis = TupleDeclareLhs::default();
        input_analysis.visit_pat(&closure.inputs[0]);

        // Perform dependency analysis on the body
        self.analysis_results = EqualityAnalysis::default();
        self.analysis_results.dependencies = input_analysis.lhs_tuple.clone();
        self.analysis_results.visit_expr(&closure.body);

        println!("Closure output dependencies: {:?}", self.analysis_results.output_dependencies);
    }
}