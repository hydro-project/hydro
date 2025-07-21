use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::{DefaultHasher, Hash, Hasher};

use hydro_lang::ir::{HydroLeaf, HydroNode, traverse_dfir};
use hydro_lang::location::LocationId;
use syn::visit::Visit;

use crate::parse_results::{NetworkType, get_network_type};
use crate::partition_syn_analysis::{AnalyzeClosure, StructOrTuple, StructOrTupleIndex};
use crate::rewrites::relevant_inputs;

// Find all inputs of a node
struct InputMetadata {
    // Const fields
    cluster_to_partition: LocationId,
    // Variables
    inputs: BTreeSet<usize>, // op_ids of cluster inputs.
}

fn input_analysis_node(
    node: &mut HydroNode,
    next_stmt_id: &mut usize,
    metadata: &mut InputMetadata,
) {
    match get_network_type(node, &metadata.cluster_to_partition) {
        Some(NetworkType::Recv) | Some(NetworkType::SendRecv) => {
            metadata.inputs.insert(*next_stmt_id);
        }
        _ => {}
    }
}

fn input_analysis(ir: &mut [HydroLeaf], cluster_to_partition: &LocationId) -> BTreeSet<usize> {
    let mut input_metadata = InputMetadata {
        cluster_to_partition: cluster_to_partition.clone(),
        inputs: BTreeSet::new(),
    };
    traverse_dfir(
        ir,
        |_, _| {},
        |node, next_stmt_id| {
            input_analysis_node(node, next_stmt_id, &mut input_metadata);
        },
    );

    input_metadata.inputs
}

pub struct InputDependencyMetadata {
    // Const fields
    pub cluster_to_partition: LocationId,
    pub inputs: BTreeSet<usize>,
    // Variables
    pub optimistic_phase: bool, /* If true, tuple intersection continues even if one side does not exist */
    pub input_taint: BTreeMap<usize, BTreeSet<usize>>, /* op_id -> set of input op_ids that taint this node */
    pub input_dependencies: BTreeMap<usize, BTreeMap<usize, StructOrTuple>>, /* op_id -> (input op_id -> index of input in output) */
    pub syn_analysis: BTreeMap<usize, StructOrTuple>, /* Cached results for analyzing f for each operator */
}

impl Hash for InputDependencyMetadata {
    // Only consider input_taint and input_dependencies
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.input_taint.hash(state);
        self.input_dependencies.hash(state);
    }
}

fn union(
    tuple1: Option<&StructOrTuple>,
    tuple1index: &StructOrTupleIndex,
    tuple2: Option<&StructOrTuple>,
    tuple2index: &StructOrTupleIndex,
) -> Option<StructOrTuple> {
    if let (Some(t1), Some(t2)) = (tuple1, tuple2) {
        if let Some(t1_child) = t1.get_dependencies(tuple1index) {
            if let Some(t2_child) = t2.get_dependencies(tuple2index) {
                return StructOrTuple::union(&t1_child, &t2_child);
            }
        }
    }
    None
}

fn input_dependency_analysis_node(
    node: &mut HydroNode,
    next_stmt_id: &mut usize,
    metadata: &mut InputDependencyMetadata,
    cycle_source_to_sink_input: &HashMap<usize, usize>,
) {
    // Filter unrelated nodes
    if metadata.cluster_to_partition != *node.metadata().location_kind.root() {
        return;
    }

    println!("Analyzing node {} {:?}", next_stmt_id, node.print_root());

    let parent_ids = match node {
        HydroNode::CycleSource { .. } => {
            // For CycleSource, its input is its CycleSink's input. Note: assume the CycleSink is on the same cluster
            vec![*cycle_source_to_sink_input.get(next_stmt_id).unwrap()]
        }
        HydroNode::Tee { inner, .. } => {
            vec![inner.0.borrow().metadata().id.unwrap()]
        }
        _ => relevant_inputs(node.input_metadata(), &metadata.cluster_to_partition),
    };

    let InputDependencyMetadata {
        inputs,
        optimistic_phase,
        input_taint,
        input_dependencies,
        syn_analysis,
        ..
    } = metadata;

    // Calculate input taints, find parent input dependencies
    let mut parent_input_dependencies: BTreeMap<usize, BTreeMap<usize, StructOrTuple>> =
        BTreeMap::new(); // input_id -> parent position (0,1,etc) -> dependencies
    let mut parent_taints: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new(); // input_id -> parents (positions) tainted by the input
    for (index, parent_id) in parent_ids.iter().enumerate() {
        if inputs.contains(parent_id) {
            // Parent is an input
            input_taint
                .entry(*next_stmt_id)
                .or_default()
                .insert(*parent_id);
            parent_taints.entry(*parent_id).or_default().insert(index);
            parent_input_dependencies
                .entry(*parent_id)
                .or_default()
                .insert(index, StructOrTuple::new_completely_dependent());
        } else if let Some(existing_parent_taints) = input_taint.get(parent_id).cloned() {
            // Otherwise, extend the parent's
            for input in &existing_parent_taints {
                parent_taints.entry(*input).or_default().insert(index);
            }
            input_taint
                .entry(*next_stmt_id)
                .or_default()
                .extend(existing_parent_taints);
            if let Some(parent_dependencies) = input_dependencies.get(parent_id) {
                // If the parent has dependencies for the input it's tainted by, add them
                for (input_id, parent_dependencies_on_input) in parent_dependencies {
                    parent_input_dependencies
                        .entry(*input_id)
                        .or_default()
                        .insert(index, parent_dependencies_on_input.clone());
                }
            }
        }

        // Difference & AntiJoin have 2 parents, but the 2nd parent influences neither its taint nor dependencies, so don't consider it
        match node {
            HydroNode::Difference { .. } | HydroNode::AntiJoin { .. } => {
                break;
            }
            _ => {}
        }
    }
    println!("Parents of node {}: {:?}", next_stmt_id, parent_ids);
    println!(
        "Parent input dependencies for node {}: {:?}",
        next_stmt_id, parent_input_dependencies
    );

    // Calculate input dependencies
    let input_taint_entry = input_taint.entry(*next_stmt_id).or_default();
    let input_dependencies_entry = input_dependencies.entry(*next_stmt_id).or_default();
    match &node {
        // 1:1 to parent
        HydroNode::CycleSource { .. }
        | HydroNode::Tee { .. }
        | HydroNode::Persist { .. }
        | HydroNode::Unpersist { .. }
        | HydroNode::Delta { .. }
        | HydroNode::ResolveFutures { .. }
        | HydroNode::ResolveFuturesOrdered { .. }
        | HydroNode::DeferTick { .. }
        | HydroNode::Unique { .. }
        | HydroNode::Sort { .. }
        | HydroNode::Difference { .. } // [a,b,c] difference [c,d] = [a,b]. Since only a subset of the 1st input is taken, we only care about its dependencies
        | HydroNode::AntiJoin { .. } // [(a,1),(b,2)] anti-join [a] = [(b,2)]. Similar to Difference
        | HydroNode::Filter { .. } // Although it contains a function f, the output is just a subset of the input, so just inherit from the parent
        | HydroNode::Inspect { .. }
        | HydroNode::Network { .. } => {
            // For each input the first (and potentially only) parent depends on, take its dependency
            for input_id in input_taint_entry.iter() {
                if let Some(parent_dependencies_on_input) = parent_input_dependencies.get(input_id) {
                    if let Some(parent_dependency) = parent_dependencies_on_input.get(&0) {
                        input_dependencies_entry.insert(*input_id, parent_dependency.clone());
                        continue;
                    }
                }
                // Parent is tainted by input but has no dependencies, delete
                input_dependencies_entry.remove(input_id);
            }
        }
        // Alters parent in a predicatable way
        HydroNode::Chain { .. } => {
            assert_eq!(parent_ids.len(), 2, "Node {:?} has the wrong number of parents.", node);
            // [a,b] chain [c,d] = [a,b,c,d]. Take the intersection of dependencies of the two parents for each input. If only one parent is tainted, then just take that dependency
            for (input_id, parent_positions) in parent_taints {
                if let Some(parent_dependencies_on_input) = parent_input_dependencies.get(&input_id) {
                    let num_tainted_parents = parent_positions.len();
                    // For each tainted parent, see if we can find its dependency (flatten to remove None)
                    let parent_dependency_tuples = parent_positions.into_iter().filter_map(|pos| parent_dependencies_on_input.get(&pos)).cloned().collect::<Vec<StructOrTuple>>();
                    if let Some(intersection) = StructOrTuple::intersect_tuples(&parent_dependency_tuples) {
                        // Only accept the dependency if each tainted parent contributed, or if we're in the optimistic phase
                        if parent_dependency_tuples.len() == num_tainted_parents || *optimistic_phase {
                            input_dependencies_entry.insert(input_id, intersection);
                            continue;
                        }
                    }
                }
                // At least one parent is tainted but has no dependencies, or there's no overlap, remove
                input_dependencies_entry.remove(&input_id);
            }
        }
        HydroNode::CrossProduct { .. }
        | HydroNode::CrossSingleton { .. } => {
            assert_eq!(parent_ids.len(), 2, "Node {:?} has the wrong number of parents.", node);
            // [a,b] cross product [c,d] = [(a,c), (a,d), (b,c), (b,d)]
            for input_id in input_taint_entry.iter() {
                if let Some(parent_dependencies_on_input) = parent_input_dependencies.get(input_id) {
                    let mut new_dependency = StructOrTuple::default();
                    if let Some(parent1_dependency) = parent_dependencies_on_input.get(&0) {
                        new_dependency.set_dependencies(&vec!["0".to_string()], parent1_dependency, &vec![]);
                    }
                    if let Some(parent2_dependency) = parent_dependencies_on_input.get(&1) {
                        new_dependency.set_dependencies(&vec!["1".to_string()], parent2_dependency, &vec![]);
                    }
                    if !new_dependency.is_empty() {
                        input_dependencies_entry.insert(*input_id, new_dependency);
                        continue;
                    }
                }
                // At least one parent has no dependencies or there's no overlap, delete
                input_dependencies_entry.remove(input_id);
            }
        }
        HydroNode::Join { .. } => {
            assert_eq!(parent_ids.len(), 2, "Node {:?} has the wrong number of parents.", node);
            // [(a,b)] join [(a,c)] = [(a,(b,c)]
            for input_id in input_taint_entry.iter() {
                if let Some(parent_dependencies_on_input) = parent_input_dependencies.get(input_id) {
                    let mut new_dependency = StructOrTuple::default();
                    // Set a to dependencies from either of the parents' 0th index
                    if let Some(union) = union(parent_dependencies_on_input.get(&0), &vec!["0".to_string()], parent_dependencies_on_input.get(&1), &vec!["0".to_string()]) {
                        new_dependency.set_dependencies(&vec!["0".to_string()], &union, &vec![]);
                    }
                    // Set b to 1st index of parent 0
                    if let Some(parent1_dependency) = parent_dependencies_on_input.get(&0) {
                        new_dependency.set_dependencies(&vec!["1".to_string(),"0".to_string()], parent1_dependency, &vec!["1".to_string()]);
                    }
                    // Set c to 1st index of parent 1
                    if let Some(parent2_dependency) = parent_dependencies_on_input.get(&1) {
                        new_dependency.set_dependencies(&vec!["1".to_string(),"1".to_string()], parent2_dependency, &vec!["1".to_string()]);
                    }
                    if !new_dependency.is_empty() {
                        input_dependencies_entry.insert(*input_id, new_dependency);
                        continue;
                    }
                }
                // At least one parent has no dependencies or there's no overlap, delete
                input_dependencies_entry.remove(input_id);
            }
        }
        HydroNode::Enumerate { .. } => {
            assert_eq!(parent_ids.len(), 1, "Node {:?} has the wrong number of parents.", node);
            // enumerate [(a,b)] = [(0,a),(1,b)]
            for input_id in input_taint_entry.iter() {
                if let Some(parent_dependencies_on_input) = parent_input_dependencies.get(input_id) {
                    if let Some(parent_dependency) = parent_dependencies_on_input.get(&0) {
                        // Set the 1st index to the parent's dependency
                        let mut new_dependency = StructOrTuple::default();
                        new_dependency.set_dependencies(&vec!["1".to_string()], parent_dependency, &vec![]);
                        input_dependencies_entry.insert(*input_id, new_dependency);
                        continue;
                    }
                }
                // Parent is tainted by input but has no dependencies, delete
                input_dependencies_entry.remove(input_id);
            }
        }
        // Based on f
        HydroNode::Map { f, .. }
        | HydroNode::FilterMap { f, .. } => {
            assert_eq!(parent_ids.len(), 1, "Node {:?} has the wrong number of parents.", node);
            // Analyze if we haven't yet
            let syn_analysis_results = syn_analysis.entry(*next_stmt_id).or_insert_with(|| {
                let mut analyzer = AnalyzeClosure::default();
                analyzer.visit_expr(&f.0);

                // Keep topmost none field if this is filter_map
                let keep_topmost_none_fields = matches!(node, HydroNode::FilterMap { .. });
                analyzer.output_dependencies.remove_none_fields(keep_topmost_none_fields).unwrap_or_default()
            });
            for input_id in input_taint_entry.iter() {
                if let Some(parent_dependencies_on_input) = parent_input_dependencies.get(input_id) {
                    if let Some(parent_dependency) = parent_dependencies_on_input.get(&0) {
                        // Project the parent's dependencies based on how f transforms the output
                        if let Some(projected_dependencies) = StructOrTuple::project_parent(parent_dependency, syn_analysis_results) {
                            println!("Node {:?} input {:?} has projected dependencies: {:?}", next_stmt_id, input_id, projected_dependencies);
                            input_dependencies_entry.insert(*input_id, projected_dependencies);
                            continue;
                        }
                    }
                }
                // Parent is tainted by input but has no dependencies, delete
                input_dependencies_entry.remove(input_id);
            }
        }
        // Only the key is preserved
        HydroNode::ReduceKeyed { .. }
        | HydroNode::FoldKeyed { .. } => {
            assert_eq!(parent_ids.len(), 1, "Node {:?} has the wrong number of parents.", node);
            for input_id in input_taint_entry.iter() {
                if let Some(parent_dependencies_on_input) = parent_input_dependencies.get(input_id) {
                    if let Some(parent_dependency) = parent_dependencies_on_input.get(&0) {
                        // Inherit only the 0th index of the parent (the key)
                        let mut new_dependency = StructOrTuple::default();
                        new_dependency.set_dependencies(&vec!["0".to_string()], parent_dependency, &vec!["0".to_string()]);
                        input_dependencies_entry.insert(*input_id, new_dependency);
                        continue;
                    }
                }
                // Parent is tainted by input but has no dependencies, delete
                input_dependencies_entry.remove(input_id);
            }
        }
        // No dependencies on the parent (or no parent)
        HydroNode::Reduce { .. }
        | HydroNode::Fold { .. }
        | HydroNode::FlatMap { .. }
        | HydroNode::Source { .. } => {
            input_dependencies_entry.clear();
        }
        HydroNode::Placeholder
        | HydroNode::Counter { .. } => {
            panic!("Unexpected node type {:?} in input dependency analysis.", node);
        }
    }
    println!(
        "Input dependencies for node {}: {:?}",
        next_stmt_id, input_dependencies_entry
    );
}

fn input_dependency_analysis(
    ir: &mut [HydroLeaf],
    cluster_to_partition: &LocationId,
    cycle_source_to_sink_input: &HashMap<usize, usize>,
) -> InputDependencyMetadata {
    let mut metadata = InputDependencyMetadata {
        cluster_to_partition: cluster_to_partition.clone(),
        inputs: input_analysis(ir, cluster_to_partition),
        optimistic_phase: true,
        input_taint: BTreeMap::new(),
        input_dependencies: BTreeMap::new(),
        syn_analysis: BTreeMap::new(),
    };

    println!("\nBegin input dependency analysis");

    let mut num_iters = 0;
    let mut prev_hash = None;
    loop {
        println!("Input dependency analysis iteration {}", num_iters);

        traverse_dfir(
            ir,
            |_, _| {}, // Don't need to analyze leaves since they don't output anyway
            |node, next_stmt_id| {
                input_dependency_analysis_node(
                    node,
                    next_stmt_id,
                    &mut metadata,
                    cycle_source_to_sink_input,
                );
            },
        );

        // Check if we've hit fixpoint
        let mut hasher = DefaultHasher::new();
        metadata.hash(&mut hasher);
        let hash = hasher.finish();
        if let Some(prev) = prev_hash {
            if prev == hash {
                if metadata.optimistic_phase {
                    println!("Optimistic phase reached fixpoint, starting pessimistic phase");
                    metadata.optimistic_phase = false;
                } else {
                    break;
                }
            }
        }
        prev_hash = Some(hash);
        num_iters += 1;
    }

    metadata
}

/// If the tuple with the given id is tainted but has no dependencies from some input, return None
fn get_inputs_and_dependencies(
    input_taint: &BTreeMap<usize, BTreeSet<usize>>,
    input_dependencies: &BTreeMap<usize, BTreeMap<usize, StructOrTuple>>,
    id: usize,
    index: &StructOrTupleIndex,
) -> Option<(Vec<usize>, Vec<StructOrTuple>)> {
    let mut ordered_input = vec![];
    let mut ordered_dependencies = vec![];

    if let Some(taints) = input_taint.get(&id) {
        for input in taints {
            if let Some(dependency) = input_dependencies
                .get(&id)
                .and_then(|map| map.get(input))
                .and_then(|tuple| tuple.get_dependencies(index))
            {
                ordered_input.push(*input);
                ordered_dependencies.push(dependency);
            } else {
                // Parent is tainted but has no dependencies, cannot partition
                return None;
            }
        }
    }

    Some((ordered_input, ordered_dependencies))
}

fn partitioning_constraint_analysis_node(
    node: &mut HydroNode,
    next_stmt_id: &mut usize,
    dependency_metadata: &InputDependencyMetadata,
    possible_partitionings: &mut BTreeMap<usize, BTreeSet<BTreeMap<usize, StructOrTupleIndex>>>, /* op -> set of partitioning requirements (input -> indices), one for each input taint. Empty set = cannot partition */
) {
    let InputDependencyMetadata {
        cluster_to_partition,
        input_taint,
        input_dependencies,
        ..
    } = dependency_metadata;

    // If this node is tainted by an input (otherwise we don't care)
    if input_taint.contains_key(next_stmt_id) {
        let parent_ids = relevant_inputs(node.input_metadata(), cluster_to_partition);

        // If there's at least 1 parent that is not tainted by any input, then we can always partition
        if parent_ids
            .iter()
            .map(|id| input_taint.get(id))
            .any(|taint| taint.is_none())
        {
            return;
        }

        let mut ordered_inputs = vec![];
        let mut ordered_dependencies = vec![];
        match node {
            HydroNode::Difference { .. } => {
                // Get the dependencies from the entirety of parent 0 and parent 1
                // Only allow partitioning if both have dependencies (the earlier if statement already checks that that both are tainted)
                if let Some((parent0_inputs, parent0_dependencies)) = get_inputs_and_dependencies(
                    input_taint,
                    input_dependencies,
                    parent_ids[0],
                    &vec![],
                ) {
                    if let Some((parent1_inputs, parent1_dependencies)) =
                        get_inputs_and_dependencies(
                            input_taint,
                            input_dependencies,
                            parent_ids[1],
                            &vec![],
                        )
                    {
                        ordered_inputs.extend(parent0_inputs);
                        ordered_dependencies.extend(parent0_dependencies);
                        ordered_inputs.extend(parent1_inputs);
                        ordered_dependencies.extend(parent1_dependencies);
                    }
                }
            }
            HydroNode::AntiJoin { .. } => {
                // Get the dependencies from field 0 of parent 0 and the entirety of parent 1
                // Only allow partitioning if both have dependencies (the earlier if statement already checks that that both are tainted)
                if let Some((parent0_inputs, parent0_dependencies)) = get_inputs_and_dependencies(
                    input_taint,
                    input_dependencies,
                    parent_ids[0],
                    &vec!["0".to_string()],
                ) {
                    if let Some((parent1_inputs, parent1_dependencies)) =
                        get_inputs_and_dependencies(
                            input_taint,
                            input_dependencies,
                            parent_ids[1],
                            &vec![],
                        )
                    {
                        ordered_inputs.extend(parent0_inputs);
                        ordered_dependencies.extend(parent0_dependencies);
                        ordered_inputs.extend(parent1_inputs);
                        ordered_dependencies.extend(parent1_dependencies);
                    }
                }
            }
            HydroNode::Join { .. } => {
                // Get the dependencies from field 0 of parent 0 and parent 1
                // Only allow partitioning if both have dependencies (the earlier if statement already checks that that both are tainted)
                if let Some((parent0_inputs, parent0_dependencies)) = get_inputs_and_dependencies(
                    input_taint,
                    input_dependencies,
                    parent_ids[0],
                    &vec!["0".to_string()],
                ) {
                    if let Some((parent1_inputs, parent1_dependencies)) =
                        get_inputs_and_dependencies(
                            input_taint,
                            input_dependencies,
                            parent_ids[1],
                            &vec!["0".to_string()],
                        )
                    {
                        ordered_inputs.extend(parent0_inputs);
                        ordered_dependencies.extend(parent0_dependencies);
                        ordered_inputs.extend(parent1_inputs);
                        ordered_dependencies.extend(parent1_dependencies);
                    }
                }
            }
            HydroNode::ReduceKeyed { .. } | HydroNode::FoldKeyed { .. } => {
                // Can only partition on the key. The key's inherited dependencies are already in input_dependencies for this node
                if let Some((inputs, dependencies)) = get_inputs_and_dependencies(
                    input_taint,
                    input_dependencies,
                    *next_stmt_id,
                    &vec![],
                ) {
                    ordered_inputs.extend(inputs);
                    ordered_dependencies.extend(dependencies);
                }
            }
            HydroNode::Reduce { .. }
            | HydroNode::Fold { .. }
            | HydroNode::Enumerate { .. }
            | HydroNode::CrossProduct { .. }
            | HydroNode::CrossSingleton { .. } => {} // Partitioning is impossible
            _ => {
                // Doesn't impede partitioning, return
                return;
            }
        }

        // If there are no ordered_dependencies, we will insert an empty set which means that partitioning is impossible
        let intersection =
            StructOrTuple::intersect_dependencies_with_matching_fields(&ordered_dependencies);
        // Convert to set of maps
        let mut possible_partitionings_for_node = BTreeSet::new();
        for possible_set in intersection {
            let mut possible_set_mapped_to_input = BTreeMap::new();
            for (pos, dependency) in possible_set.iter().enumerate() {
                let input = ordered_inputs.get(pos).unwrap();
                possible_set_mapped_to_input.insert(*input, dependency.clone());
            }
            possible_partitionings_for_node.insert(possible_set_mapped_to_input);
        }
        possible_partitionings.insert(*next_stmt_id, possible_partitionings_for_node);
    }
}

pub fn partitioning_analysis(
    ir: &mut [HydroLeaf],
    cluster_to_partition: &LocationId,
    cycle_source_to_sink_input: &HashMap<usize, usize>,
) -> Option<Vec<BTreeMap<usize, StructOrTupleIndex>>> {
    // Returns all possible partitionings
    let dependency_metadata =
        input_dependency_analysis(ir, cluster_to_partition, cycle_source_to_sink_input);
    let mut possible_partitionings = BTreeMap::new();

    println!("\nBegin partitioning constraint analysis");

    traverse_dfir(
        ir,
        |_, _| {},
        |node, next_op_id| {
            partitioning_constraint_analysis_node(
                node,
                next_op_id,
                &dependency_metadata,
                &mut possible_partitionings,
            );
        },
    );

    for (op_id, partitioning_options) in &possible_partitionings {
        println!(
            "Partitioning options for op {}: {:?}",
            op_id, partitioning_options
        );
    }

    println!("\nBegin partitioning analysis");

    // See if there are any ops preventing us from partitioning first
    let mut partitioning_impossible = false;
    for (op_id, partitioning_options) in &possible_partitionings {
        if partitioning_options.is_empty() {
            println!("No partitioning possible due to op_id {}", op_id);
            partitioning_impossible = true;
        }
    }
    if partitioning_impossible {
        return None;
    }

    // Find all possible global partitionings
    let mut prev_global_partitionings = vec![]; // vec of map from input_id to StructOrTupleIndex
    let mut next_global_partitionings = vec![];

    // Set the 1st global partitioning
    if let Some((op_id, partitioning_options)) = possible_partitionings.pop_first() {
        for partitioning in partitioning_options {
            assert!(
                !partitioning.is_empty(),
                "Op {} has partitioning constraints yet no specific input fields for those constraints",
                op_id
            );
            prev_global_partitionings.push(partitioning.clone());
        }
    } else {
        // possible_partitionings is empty
        println!("No restrictions on partitioning");
        return Some(Vec::new());
    }

    // Iterate through remaining constraints
    // Compare each prev_global_partitionings against each constraint, add to next_global_partitionings if the constraint is satisfied
    // Continue iterating after setting prev_global_partitionings to next_global_partitionings
    for (_op_id, partitioning_options) in possible_partitionings {
        for constraint in partitioning_options {
            for prev_partitioning in &prev_global_partitionings {
                // For each constraint, check if the prev_partitioning is still possible
                let mut next_partitioning = BTreeMap::new();
                let mut constraint_satisfiable = true;
                for (input, tuple_index) in prev_partitioning {
                    if let Some(constraint_index) = constraint.get(input) {
                        if let Some(index_intersection) =
                            StructOrTuple::index_intersection(tuple_index, constraint_index)
                        {
                            // Constraint satisfied
                            next_partitioning.insert(*input, index_intersection);
                        } else {
                            // No intersection, cannot partition
                            constraint_satisfiable = false;
                            break;
                        }
                    } else {
                        // No constraint on this input, keep old constraints
                        next_partitioning.insert(*input, tuple_index.clone());
                    }
                }

                if constraint_satisfiable {
                    next_global_partitionings.push(next_partitioning);
                }
            }
        }

        // Update prev partitionings before next loop
        prev_global_partitionings = std::mem::take(&mut next_global_partitionings);
    }

    // We don't know which partitioning is "better", so just return one
    println!(
        "Found {} possible global partitionings",
        prev_global_partitionings.len()
    );
    for (partitioning_index, partitioning) in prev_global_partitionings.iter().enumerate() {
        println!("Partitioning {}: {:?}", partitioning_index, partitioning);
    }
    if prev_global_partitionings.is_empty() {
        return None; // No possible partitioning
    }
    Some(prev_global_partitionings)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet, HashMap};

    use hydro_lang::deploy::HydroDeploy;
    use hydro_lang::ir::deep_clone;
    use hydro_lang::location::LocationId;
    use hydro_lang::rewrites::persist_pullup::persist_pullup;
    use hydro_lang::{Bounded, FlowBuilder, Location, NoOrder, Stream};
    use stageleft::q;

    use crate::partition_node_analysis::{
        InputDependencyMetadata, input_dependency_analysis, partitioning_analysis,
    };
    use crate::partition_syn_analysis::{StructOrTuple, StructOrTupleIndex};
    use crate::repair::{cycle_source_to_sink_input, inject_id, inject_location};

    fn test_input(
        builder: FlowBuilder<'_>,
        cluster_to_partition: LocationId,
        expected_taint: BTreeMap<usize, BTreeSet<usize>>,
        expected_dependencies: BTreeMap<usize, BTreeMap<usize, StructOrTuple>>,
    ) {
        let mut cycle_data = HashMap::new();
        let built = builder
            .optimize_with(persist_pullup)
            .optimize_with(inject_id)
            .optimize_with(|ir| {
                cycle_data = cycle_source_to_sink_input(ir);
                inject_location(ir, &cycle_data);
            })
            .into_deploy::<HydroDeploy>();
        let mut ir = deep_clone(built.ir());
        let InputDependencyMetadata {
            input_taint: actual_taint,
            input_dependencies: actual_dependencies,
            ..
        } = input_dependency_analysis(&mut ir, &cluster_to_partition, &cycle_data);

        println!("Actual taint: {:?}", actual_taint);
        println!("Actual dependencies: {:?}", actual_dependencies);

        assert_eq!(actual_taint, expected_taint);
        assert_eq!(actual_dependencies, expected_dependencies);
    }

    fn test_input_partitionable(
        builder: FlowBuilder<'_>,
        cluster_to_partition: LocationId,
        expected_partitionings: Option<Vec<BTreeMap<usize, StructOrTupleIndex>>>,
    ) {
        let mut cycle_data = HashMap::new();
        let built = builder
            .optimize_with(persist_pullup)
            .optimize_with(inject_id)
            .optimize_with(|ir| {
                cycle_data = cycle_source_to_sink_input(ir);
                inject_location(ir, &cycle_data);
            })
            .into_deploy::<HydroDeploy>();
        let mut ir = deep_clone(built.ir());
        let partitioning = partitioning_analysis(&mut ir, &cluster_to_partition, &cycle_data);

        assert_eq!(partitioning, expected_partitionings);
    }

    #[test]
    fn test_map() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode_anonymous(&cluster2)
            .map(q!(|(a, b)| (b, a + 2)))
            .for_each(q!(|(b, a2)| {
                println!("b: {}, a+2: {}", b, a2);
            }));

        let expected_taint = BTreeMap::from([
            (3, BTreeSet::from([])),  // Network
            (4, BTreeSet::from([3])), /* The implicit map following Network, imposed by broadcast_bincode_anonymous */
            (5, BTreeSet::from([3])), // The operator being tested
        ]);

        let mut implicit_map_dependencies = StructOrTuple::default();
        implicit_map_dependencies.add_dependency(&vec![], vec!["1".to_string()]);
        let mut map_expected_dependencies = StructOrTuple::default();
        map_expected_dependencies.add_dependency(
            &vec!["0".to_string()],
            vec!["1".to_string(), "1".to_string()],
        );

        let expected_dependencies = BTreeMap::from([
            (3, BTreeMap::new()),
            (4, BTreeMap::from([(3, implicit_map_dependencies)])),
            (5, BTreeMap::from([(3, map_expected_dependencies)])),
        ]);

        test_input(
            builder,
            cluster2.id(),
            expected_taint,
            expected_dependencies,
        );
    }

    #[test]
    fn test_map_partitionable() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode_anonymous(&cluster2)
            .map(q!(|(a, b)| (b, a + 2)))
            .for_each(q!(|(b, a2)| {
                println!("b: {}, a+2: {}", b, a2);
            }));

        let expected_partitionings = Some(Vec::new());
        test_input_partitionable(builder, cluster2.id(), expected_partitionings);
    }

    #[test]
    fn test_map_complex() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        cluster1
            .source_iter(q!([(1, (2, (3, 4)))]))
            .broadcast_bincode_anonymous(&cluster2)
            .map(q!(|(a, b)| (b.1, a, b.0 - a)))
            .map(q!(|(b1, _a, b0a)| (b0a, b1.0)))
            .for_each(q!(|(b0a, b10)| {
                println!("b.0 - a: {}, b.1.0: {}", b0a, b10);
            }));

        let expected_taint = BTreeMap::from([
            (3, BTreeSet::from([])),  // Network
            (4, BTreeSet::from([3])), /* The implicit map following Network, imposed by broadcast_bincode_anonymous */
            (5, BTreeSet::from([3])), // map 1
            (6, BTreeSet::from([3])), // map 2
        ]);

        let mut implicit_map_dependencies = StructOrTuple::default();
        implicit_map_dependencies.add_dependency(&vec![], vec!["1".to_string()]);
        let mut map1_expected_dependencies = StructOrTuple::default();
        map1_expected_dependencies.add_dependency(
            &vec!["0".to_string()],
            vec!["1".to_string(), "1".to_string(), "1".to_string()],
        );
        map1_expected_dependencies.add_dependency(
            &vec!["1".to_string()],
            vec!["1".to_string(), "0".to_string()],
        );
        let mut map2_expected_dependencies = StructOrTuple::default();
        map2_expected_dependencies.add_dependency(
            &vec!["1".to_string()],
            vec![
                "1".to_string(),
                "1".to_string(),
                "1".to_string(),
                "0".to_string(),
            ],
        );

        let expected_dependencies = BTreeMap::from([
            (3, BTreeMap::new()),
            (4, BTreeMap::from([(3, implicit_map_dependencies)])),
            (5, BTreeMap::from([(3, map1_expected_dependencies)])),
            (6, BTreeMap::from([(3, map2_expected_dependencies)])),
        ]);

        test_input(
            builder,
            cluster2.id(),
            expected_taint,
            expected_dependencies,
        );
    }

    #[test]
    fn test_filter_map() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode_anonymous(&cluster2)
            .filter_map(q!(|(a, b)| {
                if a > 1 {
                    Some((b, a + 2))
                } else {
                    None
                }
            }))
            .for_each(q!(|(b, a2)| {
                println!("b: {}, a+2: {}", b, a2);
            }));

        let expected_taint = BTreeMap::from([
            (3, BTreeSet::from([])),  // Network
            (4, BTreeSet::from([3])), /* The implicit map following Network, imposed by broadcast_bincode_anonymous */
            (5, BTreeSet::from([3])), // The operator being tested
        ]);

        let mut implicit_map_dependencies = StructOrTuple::default();
        implicit_map_dependencies.add_dependency(&vec![], vec!["1".to_string()]);
        let mut map_expected_dependencies = StructOrTuple::default();
        map_expected_dependencies.add_dependency(
            &vec!["0".to_string()],
            vec!["1".to_string(), "1".to_string()],
        );

        let expected_dependencies = BTreeMap::from([
            (3, BTreeMap::new()),
            (4, BTreeMap::from([(3, implicit_map_dependencies)])),
            (5, BTreeMap::from([(3, map_expected_dependencies)])),
        ]);

        test_input(
            builder,
            cluster2.id(),
            expected_taint,
            expected_dependencies,
        );
    }

    #[test]
    fn test_filter_map_remove_none() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode_anonymous(&cluster2)
            .filter_map(q!(|(a, b)| {
                if a > 1 {
                    Some((None, a + 2))
                } 
                else if a < 1 {
                    Some((Some(b), a + 2))
                }
                else {
                    None
                }
            }))
            .for_each(q!(|(none, a2)| {
                println!("None: {:?}, a+2: {}", none, a2);
            }));

        let expected_taint = BTreeMap::from([
            (3, BTreeSet::from([])),  // Network
            (4, BTreeSet::from([3])), /* The implicit map following Network, imposed by broadcast_bincode_anonymous */
            (5, BTreeSet::from([3])), // The operator being tested
        ]);

        let mut implicit_map_dependencies = StructOrTuple::default();
        implicit_map_dependencies.add_dependency(&vec![], vec!["1".to_string()]);

        let expected_dependencies = BTreeMap::from([
            (3, BTreeMap::new()),
            (4, BTreeMap::from([(3, implicit_map_dependencies)])),
            (5, BTreeMap::new()),
        ]);

        test_input(
            builder,
            cluster2.id(),
            expected_taint,
            expected_dependencies,
        );
    }

    #[test]
    fn test_delta() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        unsafe {
            cluster1
                .source_iter(q!([(1, 2)]))
                .broadcast_bincode_anonymous(&cluster2)
                .tick_batch(&cluster2.tick())
                .delta()
                .all_ticks()
                .for_each(q!(|(a, b)| {
                    println!("a: {}, b: {}", a, b);
                }));
        }

        let expected_taint = BTreeMap::from([
            (3, BTreeSet::from([])),  // Network
            (4, BTreeSet::from([3])), /* The implicit map following Network, imposed by broadcast_bincode_anonymous */
            (5, BTreeSet::from([3])), // The operator being tested
        ]);

        let mut implicit_map_dependencies = StructOrTuple::default();
        implicit_map_dependencies.add_dependency(&vec![], vec!["1".to_string()]);

        let expected_dependencies = BTreeMap::from([
            (3, BTreeMap::new()),
            (4, BTreeMap::from([(3, implicit_map_dependencies.clone())])),
            (5, BTreeMap::from([(3, implicit_map_dependencies)])), /* No dependency changes from parent */
        ]);

        test_input(
            builder,
            cluster2.id(),
            expected_taint,
            expected_dependencies,
        );
    }

    #[test]
    fn test_delta_partitionable() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        unsafe {
            cluster1
                .source_iter(q!([(1, 2)]))
                .broadcast_bincode_anonymous(&cluster2)
                .tick_batch(&cluster2.tick())
                .delta()
                .all_ticks()
                .for_each(q!(|(a, b)| {
                    println!("a: {}, b: {}", a, b);
                }));
        }

        let expected_partitionings = Some(Vec::new()); // No partitioning constraints
        test_input_partitionable(builder, cluster2.id(), expected_partitionings);
    }

    #[test]
    fn test_chain() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        let input = cluster1
            .source_iter(q!([(1, (2, 3))]))
            .broadcast_bincode_anonymous(&cluster2);
        let stream1 = input.clone().map(q!(|(a, b)| (b, a + 2)));
        let stream2 = input.map(q!(|(a, b)| ((b.1, b.1), a + 3)));
        let tick = cluster2.tick();
        unsafe {
            stream2
                .tick_batch(&tick)
                .chain(stream1.tick_batch(&tick))
                .all_ticks()
                .for_each(q!(|((x, b1), y)| {
                    println!("x: {}, b.1: {}, y: {}", x, b1, y);
                }));
        }

        let expected_taint = BTreeMap::from([
            (3, BTreeSet::from([])),  // Network
            (4, BTreeSet::from([3])), /* The implicit map following Network, imposed by broadcast_bincode_anonymous */
            (5, BTreeSet::from([3])), // tee on the input
            (6, BTreeSet::from([3])), // stream2's map
            (7, BTreeSet::from([3])), // tee on the input
            (8, BTreeSet::from([3])), // stream1's map
            (9, BTreeSet::from([3])), // chain
        ]);

        let mut implicit_map_dependencies = StructOrTuple::default();
        implicit_map_dependencies.add_dependency(&vec![], vec!["1".to_string()]);
        let mut stream1_map_dependencies = StructOrTuple::default();
        stream1_map_dependencies.add_dependency(
            &vec!["0".to_string()],
            vec!["1".to_string(), "1".to_string()],
        );
        let mut stream2_map_dependencies = StructOrTuple::default();
        stream2_map_dependencies.add_dependency(
            &vec!["0".to_string(), "0".to_string()],
            vec!["1".to_string(), "1".to_string(), "1".to_string()],
        );
        stream2_map_dependencies.add_dependency(
            &vec!["0".to_string(), "1".to_string()],
            vec!["1".to_string(), "1".to_string(), "1".to_string()],
        );
        let mut chain_dependencies = StructOrTuple::default();
        chain_dependencies.add_dependency(
            &vec!["0".to_string(), "1".to_string()],
            vec!["1".to_string(), "1".to_string(), "1".to_string()],
        );

        let expected_dependencies = BTreeMap::from([
            (3, BTreeMap::new()),
            (4, BTreeMap::from([(3, implicit_map_dependencies.clone())])),
            (5, BTreeMap::from([(3, implicit_map_dependencies.clone())])),
            (6, BTreeMap::from([(3, stream2_map_dependencies)])),
            (7, BTreeMap::from([(3, implicit_map_dependencies)])),
            (8, BTreeMap::from([(3, stream1_map_dependencies)])),
            (9, BTreeMap::from([(3, chain_dependencies)])),
        ]);

        test_input(
            builder,
            cluster2.id(),
            expected_taint,
            expected_dependencies,
        );
    }

    #[test]
    fn test_chain_partitionable() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        let input = cluster1
            .source_iter(q!([(1, (2, 3))]))
            .broadcast_bincode_anonymous(&cluster2);
        let stream1 = input.clone().map(q!(|(a, b)| (b, a + 2)));
        let stream2 = input.map(q!(|(a, b)| ((b.1, b.1), a + 3)));
        let tick = cluster2.tick();
        unsafe {
            stream2
                .tick_batch(&tick)
                .chain(stream1.tick_batch(&tick))
                .all_ticks()
                .for_each(q!(|((x, b1), y)| {
                    println!("x: {}, b.1: {}, y: {}", x, b1, y);
                }));
        }

        let expected_partitionings = Some(Vec::new()); // No partitioning constraints
        test_input_partitionable(builder, cluster2.id(), expected_partitionings);
    }

    #[test]
    fn test_cross_product() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        let input = cluster1
            .source_iter(q!([(1, (2, 3))]))
            .broadcast_bincode_anonymous(&cluster2);
        let stream1 = input.clone().map(q!(|(a, b)| (b, a + 2)));
        let stream2 = input.map(q!(|(a, b)| ((b.1, b.1), a + 3)));
        let tick = cluster2.tick();
        unsafe {
            stream2
                .tick_batch(&tick)
                .cross_product(stream1.tick_batch(&tick))
                .all_ticks()
                .for_each(q!(|(((b1, b1_again), a3), (b, a2))| {
                    println!("((({}, {}), {}), ({:?}, {}))", b1, b1_again, a3, b, a2);
                }));
        }

        let expected_taint = BTreeMap::from([
            (3, BTreeSet::from([])),  // Network
            (4, BTreeSet::from([3])), /* The implicit map following Network, imposed by broadcast_bincode_anonymous */
            (5, BTreeSet::from([3])), // tee on the input
            (6, BTreeSet::from([3])), // stream2's map
            (7, BTreeSet::from([3])), // tee on the input
            (8, BTreeSet::from([3])), // stream1's map
            (9, BTreeSet::from([3])), // cross_product
        ]);

        let mut implicit_map_dependencies = StructOrTuple::default();
        implicit_map_dependencies.add_dependency(&vec![], vec!["1".to_string()]);
        let mut stream1_map_dependencies = StructOrTuple::default();
        stream1_map_dependencies.add_dependency(
            &vec!["0".to_string()],
            vec!["1".to_string(), "1".to_string()],
        );
        let mut stream2_map_dependencies = StructOrTuple::default();
        stream2_map_dependencies.add_dependency(
            &vec!["0".to_string(), "0".to_string()],
            vec!["1".to_string(), "1".to_string(), "1".to_string()],
        );
        stream2_map_dependencies.add_dependency(
            &vec!["0".to_string(), "1".to_string()],
            vec!["1".to_string(), "1".to_string(), "1".to_string()],
        );
        let mut cross_product_dependencies = StructOrTuple::default();
        cross_product_dependencies.add_dependency(
            &vec!["0".to_string(), "0".to_string(), "0".to_string()],
            vec!["1".to_string(), "1".to_string(), "1".to_string()],
        );
        cross_product_dependencies.add_dependency(
            &vec!["0".to_string(), "0".to_string(), "1".to_string()],
            vec!["1".to_string(), "1".to_string(), "1".to_string()],
        );
        cross_product_dependencies.add_dependency(
            &vec!["1".to_string(), "0".to_string()],
            vec!["1".to_string(), "1".to_string()],
        );

        let expected_dependencies = BTreeMap::from([
            (3, BTreeMap::new()),
            (4, BTreeMap::from([(3, implicit_map_dependencies.clone())])),
            (5, BTreeMap::from([(3, implicit_map_dependencies.clone())])),
            (6, BTreeMap::from([(3, stream2_map_dependencies)])),
            (7, BTreeMap::from([(3, implicit_map_dependencies)])),
            (8, BTreeMap::from([(3, stream1_map_dependencies)])),
            (9, BTreeMap::from([(3, cross_product_dependencies)])),
        ]);

        test_input(
            builder,
            cluster2.id(),
            expected_taint,
            expected_dependencies,
        );
    }

    #[test]
    fn test_cross_product_partitionable() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        let input = cluster1
            .source_iter(q!([(1, (2, 3))]))
            .broadcast_bincode_anonymous(&cluster2);
        let stream1 = input.clone().map(q!(|(a, b)| (b, a + 2)));
        let stream2 = input.map(q!(|(a, b)| ((b.1, b.1), a + 3)));
        let tick = cluster2.tick();
        unsafe {
            stream2
                .tick_batch(&tick)
                .cross_product(stream1.tick_batch(&tick))
                .all_ticks()
                .for_each(q!(|(((b1, b1_again), a3), (b, a2))| {
                    println!("((({}, {}), {}), ({:?}, {}))", b1, b1_again, a3, b, a2);
                }));
        }

        let expected_partitionings = None;
        test_input_partitionable(builder, cluster2.id(), expected_partitionings);
    }

    #[test]
    fn test_join() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        let input = cluster1
            .source_iter(q!([(1, (2, 3))]))
            .broadcast_bincode_anonymous(&cluster2);
        let stream1 = input.clone().map(q!(|(a, b)| (b, a)));
        let stream2 = input.map(q!(|(a, b)| ((b.1, b.1), a + 3)));
        let tick = cluster2.tick();
        unsafe {
            stream2
                .tick_batch(&tick)
                .join(stream1.tick_batch(&tick))
                .all_ticks()
                .for_each(q!(|((b1, b1_again), (a3, a))| {
                    println!("(({}, {}), {}, {})", b1, b1_again, a3, a);
                }));
        }

        let expected_taint = BTreeMap::from([
            (3, BTreeSet::from([])),  // Network
            (4, BTreeSet::from([3])), /* The implicit map following Network, imposed by broadcast_bincode_anonymous */
            (5, BTreeSet::from([3])), // tee on the input
            (6, BTreeSet::from([3])), // stream2's map
            (7, BTreeSet::from([3])), // tee on the input
            (8, BTreeSet::from([3])), // stream1's map
            (9, BTreeSet::from([3])), // join
        ]);

        let mut implicit_map_dependencies = StructOrTuple::default();
        implicit_map_dependencies.add_dependency(&vec![], vec!["1".to_string()]);
        let mut stream1_map_dependencies = StructOrTuple::default();
        stream1_map_dependencies.add_dependency(
            &vec!["0".to_string()],
            vec!["1".to_string(), "1".to_string()],
        );
        stream1_map_dependencies.add_dependency(
            &vec!["1".to_string()],
            vec!["1".to_string(), "0".to_string()],
        );
        let mut stream2_map_dependencies = StructOrTuple::default();
        stream2_map_dependencies.add_dependency(
            &vec!["0".to_string(), "0".to_string()],
            vec!["1".to_string(), "1".to_string(), "1".to_string()],
        );
        stream2_map_dependencies.add_dependency(
            &vec!["0".to_string(), "1".to_string()],
            vec!["1".to_string(), "1".to_string(), "1".to_string()],
        );
        let mut join_dependencies = StructOrTuple::default();
        join_dependencies.add_dependency(
            &vec!["0".to_string()],
            vec!["1".to_string(), "1".to_string()],
        );
        join_dependencies.add_dependency(
            &vec!["0".to_string(), "0".to_string()],
            vec!["1".to_string(), "1".to_string(), "0".to_string()],
        ); // Technically redundant
        join_dependencies.add_dependency(
            &vec!["0".to_string(), "0".to_string()],
            vec!["1".to_string(), "1".to_string(), "1".to_string()],
        );
        join_dependencies.add_dependency(
            &vec!["0".to_string(), "1".to_string()],
            vec!["1".to_string(), "1".to_string(), "1".to_string()],
        );
        join_dependencies.add_dependency(
            &vec!["1".to_string(), "1".to_string()],
            vec!["1".to_string(), "0".to_string()],
        );

        let expected_dependencies = BTreeMap::from([
            (3, BTreeMap::new()),
            (4, BTreeMap::from([(3, implicit_map_dependencies.clone())])),
            (5, BTreeMap::from([(3, implicit_map_dependencies.clone())])),
            (6, BTreeMap::from([(3, stream2_map_dependencies)])),
            (7, BTreeMap::from([(3, implicit_map_dependencies)])),
            (8, BTreeMap::from([(3, stream1_map_dependencies)])),
            (9, BTreeMap::from([(3, join_dependencies)])),
        ]);

        test_input(
            builder,
            cluster2.id(),
            expected_taint,
            expected_dependencies,
        );
    }

    #[test]
    fn test_join_partitionable() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        let input = cluster1
            .source_iter(q!([(1, (2, 3))]))
            .broadcast_bincode_anonymous(&cluster2);
        let stream1 = input.clone().map(q!(|(a, b)| (b, a)));
        let stream2 = input.map(q!(|(a, b)| ((b.1, b.1), a + 3)));
        let tick = cluster2.tick();
        unsafe {
            stream2
                .tick_batch(&tick)
                .join(stream1.tick_batch(&tick))
                .all_ticks()
                .for_each(q!(|((b1, b1_again), (a3, a))| {
                    println!("(({}, {}), {}, {})", b1, b1_again, a3, a);
                }));
        }

        // Can either partition on b.0 or b.1, since the join is only successful when both b.0=b.1 or b.1=b.1
        let expected_partitionings = Some(vec![
            BTreeMap::from([(3, vec!["1".to_string(), "1".to_string(), "0".to_string()])]),
            BTreeMap::from([(3, vec!["1".to_string(), "1".to_string(), "1".to_string()])]),
        ]);
        test_input_partitionable(builder, cluster2.id(), expected_partitionings);
    }

    #[test]
    fn test_enumerate() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        unsafe {
            cluster1
                .source_iter(q!([(1, 2)]))
                .broadcast_bincode_anonymous(&cluster2)
                .assume_ordering()
                .enumerate()
                .for_each(q!(|(i, (a, b))| {
                    println!("i: {}, a: {}, b: {}", i, a, b);
                }));
        }

        let expected_taint = BTreeMap::from([
            (3, BTreeSet::from([])),  // Network
            (4, BTreeSet::from([3])), /* The implicit map following Network, imposed by broadcast_bincode_anonymous */
            (5, BTreeSet::from([3])), // enumerate
        ]);

        let mut implicit_map_dependencies = StructOrTuple::default();
        implicit_map_dependencies.add_dependency(&vec![], vec!["1".to_string()]);
        let mut enumerate_expected_dependencies = StructOrTuple::default();
        enumerate_expected_dependencies
            .add_dependency(&vec!["1".to_string()], vec!["1".to_string()]);

        let expected_dependencies = BTreeMap::from([
            (3, BTreeMap::new()),
            (4, BTreeMap::from([(3, implicit_map_dependencies)])),
            (5, BTreeMap::from([(3, enumerate_expected_dependencies)])),
        ]);

        test_input(
            builder,
            cluster2.id(),
            expected_taint,
            expected_dependencies,
        );
    }

    #[test]
    fn test_enumerate_partitionable() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        unsafe {
            cluster1
                .source_iter(q!([(1, 2)]))
                .broadcast_bincode_anonymous(&cluster2)
                .assume_ordering()
                .enumerate()
                .for_each(q!(|(i, (a, b))| {
                    println!("i: {}, a: {}, b: {}", i, a, b);
                }));
        }

        let expected_partitionings = None;
        test_input_partitionable(builder, cluster2.id(), expected_partitionings);
    }

    #[test]
    fn test_reduce_keyed() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        unsafe {
            cluster1
                .source_iter(q!([(1, 2)]))
                .broadcast_bincode_anonymous(&cluster2)
                .tick_batch(&cluster2.tick())
                .reduce_keyed_commutative(q!(|acc, b| *acc += b))
                .all_ticks()
                .for_each(q!(|(a, b_sum)| {
                    println!("a: {}, b_sum: {}", a, b_sum);
                }));
        }

        let expected_taint = BTreeMap::from([
            (3, BTreeSet::from([])),  // Network
            (4, BTreeSet::from([3])), /* The implicit map following Network, imposed by broadcast_bincode_anonymous */
            (5, BTreeSet::from([3])), // reduce_keyed
        ]);

        let mut implicit_map_dependencies = StructOrTuple::default();
        implicit_map_dependencies.add_dependency(&vec![], vec!["1".to_string()]);
        let mut reduce_keyed_expected_dependencies = StructOrTuple::default();
        reduce_keyed_expected_dependencies.add_dependency(
            &vec!["0".to_string()],
            vec!["1".to_string(), "0".to_string()],
        );

        let expected_dependencies = BTreeMap::from([
            (3, BTreeMap::new()),
            (4, BTreeMap::from([(3, implicit_map_dependencies)])),
            (5, BTreeMap::from([(3, reduce_keyed_expected_dependencies)])),
        ]);

        test_input(
            builder,
            cluster2.id(),
            expected_taint,
            expected_dependencies,
        );
    }

    #[test]
    fn test_reduce_keyed_partitionable() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        unsafe {
            cluster1
                .source_iter(q!([(1, 2)]))
                .broadcast_bincode_anonymous(&cluster2)
                .tick_batch(&cluster2.tick())
                .reduce_keyed_commutative(q!(|acc, b| *acc += b))
                .all_ticks()
                .for_each(q!(|(a, b_sum)| {
                    println!("a: {}, b_sum: {}", a, b_sum);
                }));
        }

        let expected_partitionings = Some(vec![BTreeMap::from([(
            3,
            vec!["1".to_string(), "0".to_string()],
        )])]);
        test_input_partitionable(builder, cluster2.id(), expected_partitionings);
    }

    #[test]
    fn test_reduce() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        unsafe {
            cluster1
                .source_iter(q!([(1, 2)]))
                .broadcast_bincode_anonymous(&cluster2)
                .tick_batch(&cluster2.tick())
                .reduce_commutative(q!(|(acc_a, acc_b), (a, b)| {
                    *acc_a += a;
                    *acc_b += b;
                }))
                .all_ticks()
                .for_each(q!(|(a_sum, b_sum)| {
                    println!("a_sum: {}, b_sum: {}", a_sum, b_sum);
                }));
        }

        let expected_taint = BTreeMap::from([
            (3, BTreeSet::from([])),  // Network
            (4, BTreeSet::from([3])), /* The implicit map following Network, imposed by broadcast_bincode_anonymous */
            (5, BTreeSet::from([3])), // reduce
        ]);

        let mut implicit_map_dependencies = StructOrTuple::default();
        implicit_map_dependencies.add_dependency(&vec![], vec!["1".to_string()]);
        let expected_dependencies = BTreeMap::from([
            (3, BTreeMap::new()),
            (4, BTreeMap::from([(3, implicit_map_dependencies)])),
            (5, BTreeMap::new()),
        ]);

        test_input(
            builder,
            cluster2.id(),
            expected_taint,
            expected_dependencies,
        );
    }

    #[test]
    fn test_reduce_partitionable() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        unsafe {
            cluster1
                .source_iter(q!([(1, 2)]))
                .broadcast_bincode_anonymous(&cluster2)
                .tick_batch(&cluster2.tick())
                .reduce_commutative(q!(|(acc_a, acc_b), (a, b)| {
                    *acc_a += a;
                    *acc_b += b;
                }))
                .all_ticks()
                .for_each(q!(|(a_sum, b_sum)| {
                    println!("a_sum: {}, b_sum: {}", a_sum, b_sum);
                }));
        }

        let expected_partitionings = None;
        test_input_partitionable(builder, cluster2.id(), expected_partitionings);
    }

    #[test]
    fn test_cycle() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        let input = cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode_anonymous(&cluster2);
        let cluster2_tick = cluster2.tick();
        let (complete_cycle, cycle) =
            cluster2_tick.cycle::<Stream<(usize, usize), _, Bounded, NoOrder>>();
        let prev_tick_input = cycle
            .clone()
            .filter(q!(|(a, _b)| *a > 2))
            .map(q!(|(a, b)| (a, b + 2)));
        unsafe {
            complete_cycle
                .complete_next_tick(prev_tick_input.chain(input.tick_batch(&cluster2_tick)));
        }
        cycle.all_ticks().for_each(q!(|(a, b)| {
            println!("a: {}, b: {}", a, b);
        }));

        let expected_taint = BTreeMap::from([
            (0, BTreeSet::from([7])),  // CycleSource
            (1, BTreeSet::from([7])),  // Tee(CycleSource)
            (2, BTreeSet::from([7])),  // filter
            (3, BTreeSet::from([7])),  // map
            (7, BTreeSet::from([])),   // Network
            (8, BTreeSet::from([7])),  // Implicit map after network
            (9, BTreeSet::from([7])),  // chain
            (10, BTreeSet::from([7])), // DeferTick
            (11, BTreeSet::from([7])), // Tee(CycleSource)
        ]);

        let mut implicit_map_dependencies = StructOrTuple::default();
        implicit_map_dependencies.add_dependency(&vec![], vec!["1".to_string()]);
        let mut cycle_dependencies = StructOrTuple::default();
        cycle_dependencies.add_dependency(
            &vec!["0".to_string()],
            vec!["1".to_string(), "0".to_string()],
        );

        let expected_dependencies = BTreeMap::from([
            (0, BTreeMap::from([(7, cycle_dependencies.clone())])),
            (1, BTreeMap::from([(7, cycle_dependencies.clone())])),
            (2, BTreeMap::from([(7, cycle_dependencies.clone())])),
            (3, BTreeMap::from([(7, cycle_dependencies.clone())])),
            (7, BTreeMap::new()),
            (8, BTreeMap::from([(7, implicit_map_dependencies)])),
            (9, BTreeMap::from([(7, cycle_dependencies.clone())])),
            (10, BTreeMap::from([(7, cycle_dependencies.clone())])),
            (11, BTreeMap::from([(7, cycle_dependencies)])),
        ]);

        test_input(
            builder,
            cluster2.id(),
            expected_taint,
            expected_dependencies,
        );
    }

    #[test]
    fn test_cycle_partitionable() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        let input = cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode_anonymous(&cluster2);
        let cluster2_tick = cluster2.tick();
        let (complete_cycle, cycle) =
            cluster2_tick.cycle::<Stream<(usize, usize), _, Bounded, NoOrder>>();
        let prev_tick_input = cycle
            .clone()
            .filter(q!(|(a, _b)| *a > 2))
            .map(q!(|(a, b)| (a, b + 2)));
        unsafe {
            complete_cycle
                .complete_next_tick(prev_tick_input.chain(input.tick_batch(&cluster2_tick)));
        }
        cycle.all_ticks().for_each(q!(|(a, b)| {
            println!("a: {}, b: {}", a, b);
        }));

        let expected_partitionings = Some(Vec::new());
        test_input_partitionable(builder, cluster2.id(), expected_partitionings);
    }

    #[test]
    fn test_nested_cycle() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        let input = cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode_anonymous(&cluster2);
        let cluster2_tick = cluster2.tick();
        let (complete_cycle1, cycle1) =
            cluster2_tick.cycle::<Stream<(usize, usize), _, Bounded, NoOrder>>();
        let (complete_cycle2, cycle2) =
            cluster2_tick.cycle::<Stream<(usize, usize), _, Bounded, NoOrder>>();
        let chained = unsafe {
            cycle1.join(input.tick_batch(&cluster2_tick))
                .map(q!(|(_, (b1,b2))| (b1,b2))) // Both values are influenced by the join with cycle2_out
                .chain(cycle2)
        };
        complete_cycle1.complete_next_tick(chained.clone());
        let cycle2_out = chained.map(q!(|(_a, b)| (b, b)));
        complete_cycle2.complete_next_tick(cycle2_out.clone());
        cycle2_out.all_ticks().for_each(q!(|(b, _)| {
            println!("b: {}", b);
        }));

        let expected_taint = BTreeMap::from([
            (0, BTreeSet::from([4])),  // CycleSource(cycle1), parent = 11
            (4, BTreeSet::from([])),   // Network
            (5, BTreeSet::from([4])),  // implicit map
            (6, BTreeSet::from([4])),  // join (0 and 5)
            (7, BTreeSet::from([4])),  // map (x,(a,b)) to (a,b)
            (8, BTreeSet::from([4])),  // CycleSource(cycle2)
            (9, BTreeSet::from([4])),  // chain
            (10, BTreeSet::from([4])), // Tee(chain)
            (11, BTreeSet::from([4])), // DeferTick(cycle1)
            (12, BTreeSet::from([4])), // Tee(chain)
            (13, BTreeSet::from([4])), // map (a,b) to (b,b)
            (14, BTreeSet::from([4])), // Tee(map)
            (15, BTreeSet::from([4])), // DeferTick(cycle2)
            (16, BTreeSet::from([4])), // Tee(map)
        ]);

        let mut implicit_map_dependencies = StructOrTuple::default();
        implicit_map_dependencies.add_dependency(&vec![], vec!["1".to_string()]);
        let mut join_dependencies = StructOrTuple::default();
        join_dependencies.add_dependency(
            &vec!["0".to_string()],
            vec!["1".to_string(), "0".to_string()],
        );
        join_dependencies.add_dependency(
            &vec!["0".to_string()],
            vec!["1".to_string(), "1".to_string()],
        );
        join_dependencies.add_dependency(
            &vec!["1".to_string(), "0".to_string()],
            vec!["1".to_string(), "1".to_string()],
        );
        join_dependencies.add_dependency(
            &vec!["1".to_string(), "1".to_string()],
            vec!["1".to_string(), "1".to_string()],
        );
        let mut other_dependencies = StructOrTuple::default();
        other_dependencies.add_dependency(
            &vec!["0".to_string()],
            vec!["1".to_string(), "1".to_string()],
        );
        other_dependencies.add_dependency(
            &vec!["1".to_string()],
            vec!["1".to_string(), "1".to_string()],
        );

        let expected_dependencies = BTreeMap::from([
            (0, BTreeMap::from([(4, other_dependencies.clone())])),
            (4, BTreeMap::new()),
            (5, BTreeMap::from([(4, implicit_map_dependencies.clone())])),
            (6, BTreeMap::from([(4, join_dependencies.clone())])),
            (7, BTreeMap::from([(4, other_dependencies.clone())])),
            (8, BTreeMap::from([(4, other_dependencies.clone())])),
            (9, BTreeMap::from([(4, other_dependencies.clone())])),
            (10, BTreeMap::from([(4, other_dependencies.clone())])),
            (11, BTreeMap::from([(4, other_dependencies.clone())])),
            (12, BTreeMap::from([(4, other_dependencies.clone())])),
            (13, BTreeMap::from([(4, other_dependencies.clone())])),
            (14, BTreeMap::from([(4, other_dependencies.clone())])),
            (15, BTreeMap::from([(4, other_dependencies.clone())])),
            (16, BTreeMap::from([(4, other_dependencies)])),
        ]);

        test_input(
            builder,
            cluster2.id(),
            expected_taint,
            expected_dependencies,
        );
    }

    #[test]
    fn test_nested_cycle_partitionable() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        let input = cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode_anonymous(&cluster2);
        let cluster2_tick = cluster2.tick();
        let (complete_cycle1, cycle1) =
            cluster2_tick.cycle::<Stream<(usize, usize), _, Bounded, NoOrder>>();
        let (complete_cycle2, cycle2) =
            cluster2_tick.cycle::<Stream<(usize, usize), _, Bounded, NoOrder>>();
        let chained = unsafe {
            cycle1.join(input.tick_batch(&cluster2_tick))
                .map(q!(|(_, (b1,b2))| (b1,b2))) // Both values are influenced by the join with cycle2_out
                .chain(cycle2)
        };
        complete_cycle1.complete_next_tick(chained.clone());
        let cycle2_out = chained.map(q!(|(_a, b)| (b, b)));
        complete_cycle2.complete_next_tick(cycle2_out.clone());
        cycle2_out.all_ticks().for_each(q!(|(b, _)| {
            println!("b: {}", b);
        }));

        // Less confusing with thought experiment: Consider input tuples (1,2) and (1,3)
        // Partitioning on b fails if cycle1 (somehow) already contains tuple (1,1)
        let expected_partitionings = Some(vec![BTreeMap::from([(
            4,
            vec!["1".to_string(), "0".to_string()],
        )])]);
        test_input_partitionable(builder, cluster2.id(), expected_partitionings);
    }

    #[test]
    fn test_source_iter() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        let input = cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode_anonymous(&cluster2);
        let tick = cluster2.tick();
        let stream1 = input.map(q!(|(a, b)| (b, a + 2)));
        let stream2 = cluster2.source_iter(q!([(3, 4)]));
        unsafe {
            stream2
                .tick_batch(&tick)
                .chain(stream1.tick_batch(&tick))
                .all_ticks()
                .for_each(q!(|_| {
                    println!("No dependencies");
                }));
        }

        let expected_taint = BTreeMap::from([
            (0, BTreeSet::from([])),  // source_iter
            (4, BTreeSet::from([])),  // Network
            (5, BTreeSet::from([4])), /* The implicit map following Network, imposed by broadcast_bincode_anonymous */
            (6, BTreeSet::from([4])), // map
            (7, BTreeSet::from([4])), // chain
        ]);

        let mut implicit_map_dependencies = StructOrTuple::default();
        implicit_map_dependencies.add_dependency(&vec![], vec!["1".to_string()]);
        let mut map_dependencies = StructOrTuple::default();
        map_dependencies.add_dependency(
            &vec!["0".to_string()],
            vec!["1".to_string(), "1".to_string()],
        );

        let expected_dependencies = BTreeMap::from([
            (0, BTreeMap::new()),
            (4, BTreeMap::new()),
            (5, BTreeMap::from([(4, implicit_map_dependencies)])),
            (6, BTreeMap::from([(4, map_dependencies.clone())])),
            (7, BTreeMap::from([(4, map_dependencies)])),
        ]);

        test_input(
            builder,
            cluster2.id(),
            expected_taint,
            expected_dependencies,
        );
    }

    #[test]
    fn test_source_iter_partitionable() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        let input = cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode_anonymous(&cluster2);
        let tick = cluster2.tick();
        let stream1 = input.map(q!(|(a, b)| (b, a + 2)));
        let stream2 = cluster2.source_iter(q!([(3, 4)]));
        unsafe {
            stream2
                .tick_batch(&tick)
                .chain(stream1.tick_batch(&tick))
                .all_ticks()
                .for_each(q!(|_| {
                    println!("No dependencies");
                }));
        }

        let expected_partitionings = Some(Vec::new());
        test_input_partitionable(builder, cluster2.id(), expected_partitionings);
    }

    #[test]
    fn test_multiple_inputs() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        let input1 = cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode_anonymous(&cluster2);
        let input2 = cluster1
            .source_iter(q!([(3, 4)]))
            .broadcast_bincode_anonymous(&cluster2);
        let tick = cluster2.tick();
        unsafe {
            let stream1 = input1.map(q!(|(a, b)| (a * 2, b))).tick_batch(&tick);
            let stream2 = input2.map(q!(|(a, b)| (-a, b))).tick_batch(&tick);
            stream2
                .clone()
                .chain(stream1.clone())
                .all_ticks()
                .for_each(q!(|_| {
                    println!("Dependent on both input1.b and input2.b");
                }));
            stream2
                .join(stream1)
                .all_ticks()
                .for_each(q!(|(_, (b1, b2))| {
                    println!("b from input 1: {}, b from input 2: {}", b1, b2);
                }));
        }

        let expected_taint = BTreeMap::from([
            (3, BTreeSet::from([])),       // input2
            (4, BTreeSet::from([3])), /* The implicit map following Network, imposed by broadcast_bincode_anonymous */
            (5, BTreeSet::from([3])), // input2's map
            (6, BTreeSet::from([3])), // Tee(input2's map)
            (10, BTreeSet::from([])), // input1
            (11, BTreeSet::from([10])), /* The implicit map following Network, imposed by broadcast_bincode_anonymous */
            (12, BTreeSet::from([10])), // input1's map
            (13, BTreeSet::from([10])), // Tee(input1's map)
            (14, BTreeSet::from([3, 10])), // chain
            (16, BTreeSet::from([3])),  // Tee(input2's map)
            (17, BTreeSet::from([10])), // Tee(input1's map)
            (18, BTreeSet::from([3, 10])), // join
        ]);

        let mut implicit_map_dependencies = StructOrTuple::default();
        implicit_map_dependencies.add_dependency(&vec![], vec!["1".to_string()]);
        let mut input_map_dependencies = StructOrTuple::default();
        input_map_dependencies.add_dependency(
            &vec!["1".to_string()],
            vec!["1".to_string(), "1".to_string()],
        );
        let mut join_input1_dependencies = StructOrTuple::default();
        join_input1_dependencies.add_dependency(
            &vec!["1".to_string(), "1".to_string()],
            vec!["1".to_string(), "1".to_string()],
        );
        let mut join_input2_dependencies = StructOrTuple::default();
        join_input2_dependencies.add_dependency(
            &vec!["1".to_string(), "0".to_string()],
            vec!["1".to_string(), "1".to_string()],
        );

        let expected_dependencies = BTreeMap::from([
            (3, BTreeMap::new()),
            (4, BTreeMap::from([(3, implicit_map_dependencies.clone())])),
            (5, BTreeMap::from([(3, input_map_dependencies.clone())])),
            (6, BTreeMap::from([(3, input_map_dependencies.clone())])),
            (10, BTreeMap::new()),
            (11, BTreeMap::from([(10, implicit_map_dependencies)])),
            (12, BTreeMap::from([(10, input_map_dependencies.clone())])),
            (13, BTreeMap::from([(10, input_map_dependencies.clone())])),
            (
                14,
                BTreeMap::from([
                    (3, input_map_dependencies.clone()),
                    (10, input_map_dependencies.clone()),
                ]),
            ),
            (16, BTreeMap::from([(3, input_map_dependencies.clone())])),
            (17, BTreeMap::from([(10, input_map_dependencies)])),
            (
                18,
                BTreeMap::from([
                    (3, join_input2_dependencies),
                    (10, join_input1_dependencies),
                ]),
            ),
        ]);

        test_input(
            builder,
            cluster2.id(),
            expected_taint,
            expected_dependencies,
        );
    }

    #[test]
    fn test_multiple_inputs_partitionable() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        let input1 = cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode_anonymous(&cluster2);
        let input2 = cluster1
            .source_iter(q!([(3, 4)]))
            .broadcast_bincode_anonymous(&cluster2);
        let tick = cluster2.tick();
        unsafe {
            let stream1 = input1.map(q!(|(a, b)| (b, a * 2))).tick_batch(&tick);
            let stream2 = input2.map(q!(|(a, b)| (b, -a))).tick_batch(&tick);
            stream2
                .clone()
                .chain(stream1.clone())
                .all_ticks()
                .for_each(q!(|_| {
                    println!("Dependent on both input1.b and input2.b");
                }));
            stream2
                .join(stream1)
                .all_ticks()
                .for_each(q!(|(_, (a1, a2))| {
                    println!("a*2 from input 1: {}, -a from input 2: {}", a1, a2);
                }));
        }

        let expected_partitioning = Some(vec![BTreeMap::from([
            (3, vec!["1".to_string(), "1".to_string()]),
            (10, vec!["1".to_string(), "1".to_string()]),
        ])]);
        test_input_partitionable(builder, cluster2.id(), expected_partitioning);
    }

    #[test]
    fn test_difference() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        let input1 = cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode_anonymous(&cluster2);
        let input2 = cluster1
            .source_iter(q!([(3, 4)]))
            .broadcast_bincode_anonymous(&cluster2);
        let tick = cluster2.tick();
        unsafe {
            input1
                .tick_batch(&tick)
                .filter_not_in(input2.tick_batch(&tick))
        }
        .all_ticks()
        .for_each(q!(|(a, b)| {
            println!("a: {}, b: {}", a, b);
        }));

        let expected_taint = BTreeMap::from([
            (3, BTreeSet::new()),      // input1
            (4, BTreeSet::from([3])), /* The implicit map following Network, imposed by broadcast_bincode_anonymous */
            (8, BTreeSet::new()),     // input2's map
            (9, BTreeSet::from([8])), /* The implicit map following Network, imposed by broadcast_bincode_anonymous */
            (10, BTreeSet::from([3])), // Difference. Isn't tainted by anti-joined parent
        ]);

        let mut implicit_map_dependencies = StructOrTuple::default();
        implicit_map_dependencies.add_dependency(&vec![], vec!["1".to_string()]);

        let expected_dependencies = BTreeMap::from([
            (3, BTreeMap::new()),
            (4, BTreeMap::from([(3, implicit_map_dependencies.clone())])),
            (8, BTreeMap::new()),
            (9, BTreeMap::from([(8, implicit_map_dependencies.clone())])),
            (10, BTreeMap::from([(3, implicit_map_dependencies)])),
        ]);

        test_input(
            builder,
            cluster2.id(),
            expected_taint,
            expected_dependencies,
        );
    }

    #[test]
    fn test_difference_partitionable() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        let input1 = cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode_anonymous(&cluster2);
        let input2 = cluster1
            .source_iter(q!([(3, 4)]))
            .broadcast_bincode_anonymous(&cluster2);
        let tick = cluster2.tick();
        unsafe {
            input1
                .tick_batch(&tick)
                .filter_not_in(input2.tick_batch(&tick))
        }
        .all_ticks()
        .for_each(q!(|(a, b)| {
            println!("a: {}, b: {}", a, b);
        }));

        let expected_partitionings = Some(vec![BTreeMap::from([
            (3, vec!["1".to_string()]),
            (8, vec!["1".to_string()]),
        ])]);
        test_input_partitionable(builder, cluster2.id(), expected_partitionings);
    }
}
