use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::{DefaultHasher, Hash, Hasher};

use hydro_lang::builder::ir::{HydroNode, HydroRoot, traverse_dfir};
use hydro_lang::location::LocationId;
use syn::visit::Visit;

use super::rewrites::{NetworkType, get_network_type, relevant_inputs};
use crate::partition_syn_analysis::{AnalyzeClosure, StructOrTuple, StructOrTupleIndex};

// Find all inputs of a node
struct InputMetadata {
    // Const fields
    location: LocationId,
    // Variables
    inputs: BTreeSet<usize>,               // op_ids of cluster inputs.
    input_parents: BTreeMap<usize, usize>, /* input op_id -> parent of the input (potentially on a different location) */
}

fn input_analysis_node(
    node: &mut HydroNode,
    next_stmt_id: &mut usize,
    metadata: &mut InputMetadata,
) {
    match get_network_type(node, metadata.location.root().raw_id()) {
        Some(NetworkType::Recv) | Some(NetworkType::SendRecv) => {
            metadata.inputs.insert(*next_stmt_id);
            metadata.input_parents.insert(
                *next_stmt_id,
                node.input_metadata().first().unwrap().op.id.unwrap(),
            );
        }
        _ => {}
    }
}

fn input_analysis(
    ir: &mut [HydroRoot],
    location: &LocationId,
) -> (BTreeSet<usize>, BTreeMap<usize, usize>) {
    let mut input_metadata = InputMetadata {
        location: location.clone(),
        inputs: BTreeSet::new(),
        input_parents: BTreeMap::new(),
    };
    traverse_dfir(
        ir,
        |_, _| {},
        |node, next_stmt_id| {
            input_analysis_node(node, next_stmt_id, &mut input_metadata);
        },
    );

    (input_metadata.inputs, input_metadata.input_parents)
}

pub struct InputDependencyMetadata {
    // Const fields
    pub location: LocationId,
    pub inputs: BTreeSet<usize>,
    pub input_parents: BTreeMap<usize, usize>,
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
    if let (Some(t1), Some(t2)) = (tuple1, tuple2)
        && let Some(t1_child) = t1.get_dependencies(tuple1index)
        && let Some(t2_child) = t2.get_dependencies(tuple2index)
    {
        return StructOrTuple::union(&t1_child, &t2_child);
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
    if metadata.location != *node.metadata().location_kind.root() {
        return;
    }

    println!("Analyzing node {} {:?}", next_stmt_id, node.print_root());

    let parent_ids = match node {
        HydroNode::CycleSource { .. } => {
            // For CycleSource, its input is its CycleSink's input. Note: assume the CycleSink is on the same cluster
            vec![*cycle_source_to_sink_input.get(next_stmt_id).unwrap()]
        }
        HydroNode::Tee { inner, .. } => {
            vec![inner.0.borrow().metadata().op.id.unwrap()]
        }
        _ => relevant_inputs(node.input_metadata(), &metadata.location),
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
        | HydroNode::Network { .. }
        | HydroNode::ExternalInput { .. } => {
            // For each input the first (and potentially only) parent depends on, take its dependency
            for input_id in input_taint_entry.iter() {
                if let Some(parent_dependencies_on_input) = parent_input_dependencies.get(input_id) &&
                    let Some(parent_dependency) = parent_dependencies_on_input.get(&0)
                {
                    input_dependencies_entry.insert(*input_id, parent_dependency.clone());
                    continue;
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
                    // For each tainted parent, see if we can find its dependency (filter_map to remove None)
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
            // [(a,b)] join [(a,c)] = [(a,(b,c))]
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
                if let Some(parent_dependencies_on_input) = parent_input_dependencies.get(input_id) &&
                    let Some(parent_dependency) = parent_dependencies_on_input.get(&0)
                {
                    // Set the 1st index to the parent's dependency
                    let mut new_dependency = StructOrTuple::default();
                    new_dependency.set_dependencies(&vec!["1".to_string()], parent_dependency, &vec![]);
                    input_dependencies_entry.insert(*input_id, new_dependency);
                    continue;
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
                if let Some(parent_dependencies_on_input) = parent_input_dependencies.get(input_id) &&
                    let Some(parent_dependency) = parent_dependencies_on_input.get(&0) &&
                        // Project the parent's dependencies based on how f transforms the output
                    let Some(projected_dependencies) = StructOrTuple::project_parent(parent_dependency, syn_analysis_results)
                {
                    println!("Node {:?} input {:?} has projected dependencies: {:?}", next_stmt_id, input_id, projected_dependencies);
                    input_dependencies_entry.insert(*input_id, projected_dependencies);
                    continue;
                }
                // Parent is tainted by input but has no dependencies, delete
                input_dependencies_entry.remove(input_id);
            }
        }
        // Only the key is preserved
        HydroNode::ReduceKeyed { .. }
        | HydroNode::FoldKeyed { .. }
        | HydroNode::ReduceKeyedWatermark { .. } => {
            for input_id in input_taint_entry.iter() {
                if let Some(parent_dependencies_on_input) = parent_input_dependencies.get(input_id)
                    && let Some(parent_dependency) = parent_dependencies_on_input.get(&0)
                {
                    // Inherit only the 0th index of the parent (the key)
                    let mut new_dependency = StructOrTuple::default();
                    new_dependency.set_dependencies(&vec!["0".to_string()], parent_dependency, &vec!["0".to_string()]);
                    input_dependencies_entry.insert(*input_id, new_dependency);
                    continue;
                }
                // Parent is tainted by input but has no dependencies, delete
                input_dependencies_entry.remove(input_id);
            }
        }
        // No dependencies on the parent (or no parent)
        HydroNode::Reduce { .. }
        | HydroNode::Fold { .. }
        | HydroNode::Scan { .. }
        | HydroNode::FlatMap { .. }
        | HydroNode::Source { .. } => {
            input_dependencies_entry.clear();
        }
        HydroNode::Placeholder
        | HydroNode::Counter { .. } => {
            panic!("Unexpected node type {:?} in input dependency analysis.", node);
        }
    }

    // If there is no input taint, remove it
    if input_taint_entry.is_empty() {
        input_taint.remove(next_stmt_id);
    }

    println!(
        "Input dependencies for node {}: {:?}",
        next_stmt_id, input_dependencies_entry
    );
}

fn input_dependency_analysis(
    ir: &mut [HydroRoot],
    location: &LocationId,
    cycle_source_to_sink_input: &HashMap<usize, usize>,
) -> InputDependencyMetadata {
    let (inputs, input_parents) = input_analysis(ir, location);
    let mut metadata = InputDependencyMetadata {
        location: location.clone(),
        inputs,
        input_parents,
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
            |_, _| {}, // Don't need to analyze roots since they don't output anyway
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
        if prev_hash.is_some_and(|prev| prev == hash) {
            if metadata.optimistic_phase {
                println!("Optimistic phase reached fixpoint, starting pessimistic phase");
                metadata.optimistic_phase = false;
            } else {
                break;
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
        location,
        input_taint,
        input_dependencies,
        ..
    } = dependency_metadata;

    // If this node is tainted by an input (otherwise we don't care)
    if input_taint.contains_key(next_stmt_id) {
        let parent_ids = relevant_inputs(node.input_metadata(), location);

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
                ) && let Some((parent1_inputs, parent1_dependencies)) =
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
            HydroNode::AntiJoin { .. } => {
                // Get the dependencies from field 0 of parent 0 and the entirety of parent 1
                // Only allow partitioning if both have dependencies (the earlier if statement already checks that that both are tainted)
                if let Some((parent0_inputs, parent0_dependencies)) = get_inputs_and_dependencies(
                    input_taint,
                    input_dependencies,
                    parent_ids[0],
                    &vec!["0".to_string()],
                ) && let Some((parent1_inputs, parent1_dependencies)) =
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
            HydroNode::Join { .. } => {
                // Get the dependencies from field 0 of parent 0 and parent 1
                // Only allow partitioning if both have dependencies (the earlier if statement already checks that that both are tainted)
                if let Some((parent0_inputs, parent0_dependencies)) = get_inputs_and_dependencies(
                    input_taint,
                    input_dependencies,
                    parent_ids[0],
                    &vec!["0".to_string()],
                ) && let Some((parent1_inputs, parent1_dependencies)) =
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
            HydroNode::ReduceKeyed { .. }
            | HydroNode::FoldKeyed { .. }
            | HydroNode::ReduceKeyedWatermark { .. } => {
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

/// Returns (all possible partitionings, inputs -> inputs' parents)
#[expect(clippy::type_complexity, reason = "internal optimization code")]
pub fn partitioning_analysis(
    ir: &mut [HydroRoot],
    location: &LocationId,
    cycle_source_to_sink_input: &HashMap<usize, usize>,
) -> Option<(
    Vec<BTreeMap<usize, StructOrTupleIndex>>,
    BTreeMap<usize, usize>,
)> {
    let dependency_metadata = input_dependency_analysis(ir, location, cycle_source_to_sink_input);
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
        return Some((Vec::new(), dependency_metadata.input_parents));
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
                // Add any new constraints not found in prev_partitioning
                for (input, constraint_index) in &constraint {
                    if !next_partitioning.contains_key(input) {
                        next_partitioning.insert(*input, constraint_index.clone());
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
    Some((prev_global_partitionings, dependency_metadata.input_parents))
}

/// Given the set of possible partitionings, select an arbitrary one to use, and find all the parents of Network nodes that need to be modified
#[expect(clippy::type_complexity, reason = "internal optimization code")]
pub fn nodes_to_partition(
    analysis_results: Option<(
        Vec<BTreeMap<usize, StructOrTupleIndex>>,
        BTreeMap<usize, usize>,
    )>,
) -> Option<HashMap<usize, StructOrTupleIndex>> {
    if let Some((possible_partitionings, input_parents)) = analysis_results {
        let default_partitioning = BTreeMap::new();
        let partitioning = possible_partitionings
            .first()
            .unwrap_or(&default_partitioning);
        let mut nodes_to_partition = HashMap::new();

        for (input, parent) in input_parents {
            if let Some(partition_index) = partitioning.get(&input) {
                nodes_to_partition.insert(parent, partition_index.clone());
            } else {
                // No restriction, partition on the entire thing
                nodes_to_partition.insert(parent, vec![]);
            }
        }

        return Some(nodes_to_partition);
    }
    None
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet, HashMap};

    use hydro_lang::builder::ir::deep_clone;
    use hydro_lang::deploy::HydroDeploy;
    use hydro_lang::live_collections::stream::NoOrder;
    use hydro_lang::location::LocationId;
    use hydro_lang::prelude::*;
    use hydro_lang::rewrites::persist_pullup::persist_pullup;
    use stageleft::q;

    use crate::debug::name_to_id_map;
    use crate::partition_node_analysis::{
        InputDependencyMetadata, input_dependency_analysis, partitioning_analysis,
    };
    use crate::partition_syn_analysis::{StructOrTuple, StructOrTupleIndex};
    use crate::repair::{cycle_source_to_sink_input, inject_id, inject_location};

    fn test_input_with_unnamed_ir_nodes(
        builder: FlowBuilder<'_>,
        location: LocationId,
        expected_taint: BTreeMap<&str, BTreeSet<&str>>,
        unnamed_expected_taint: BTreeMap<(&str, i32), BTreeSet<&str>>, /* The i32 will be added to the named ir node's ID as offset */
        expected_dependencies: BTreeMap<&str, BTreeMap<&str, StructOrTuple>>,
        unnamed_expected_dependencies: BTreeMap<(&str, i32), BTreeMap<&str, StructOrTuple>>,
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
        } = input_dependency_analysis(&mut ir, &location, &cycle_data);

        println!("Expected taint: {:?}", expected_taint);
        println!("Expected dependencies: {:?}", expected_dependencies);

        // Convert names to IDs
        let name_to_id = name_to_id_map(&mut ir);
        let expected_taint_ids = expected_taint.into_iter().map(|(k, v)| {
            println!("Converting IR name {} to ID", k);
            (
                *name_to_id.get(k).unwrap(),
                v.into_iter()
                    .map(|e| *name_to_id.get(e).unwrap())
                    .collect::<BTreeSet<_>>(),
            )
        });
        let unnamed_expected_taint_ids =
            unnamed_expected_taint.into_iter().map(|((k, offset), v)| {
                (
                    (*name_to_id.get(k).unwrap() as i32 + offset) as usize,
                    v.into_iter()
                        .map(|e| *name_to_id.get(e).unwrap())
                        .collect::<BTreeSet<_>>(),
                )
            });
        let all_expected_taint_ids = expected_taint_ids
            .chain(unnamed_expected_taint_ids)
            .collect::<BTreeMap<_, _>>();
        let expected_dependencies_ids = expected_dependencies.into_iter().map(|(k, v)| {
            let k_id = *name_to_id.get(k).unwrap();
            let v_id = v
                .into_iter()
                .map(|(inner_k, inner_v)| (*name_to_id.get(inner_k).unwrap(), inner_v.clone()))
                .collect::<BTreeMap<_, _>>();
            (k_id, v_id)
        });
        let unnamed_expected_dependencies_ids =
            unnamed_expected_dependencies
                .into_iter()
                .map(|((k, offset), v)| {
                    let k_id = (*name_to_id.get(k).unwrap() as i32 + offset) as usize;
                    let v_id = v
                        .into_iter()
                        .map(|(inner_k, inner_v)| {
                            (*name_to_id.get(inner_k).unwrap(), inner_v.clone())
                        })
                        .collect::<BTreeMap<_, _>>();
                    (k_id, v_id)
                });
        let all_expected_dependencies_ids = expected_dependencies_ids
            .chain(unnamed_expected_dependencies_ids)
            .collect::<BTreeMap<_, _>>();

        println!("Expected taint, translated: {:?}", all_expected_taint_ids);
        println!("Actual taint: {:?}", actual_taint);
        println!(
            "Expected dependencies, translated: {:?}",
            all_expected_dependencies_ids
        );
        println!("Actual dependencies: {:?}", actual_dependencies);

        assert_eq!(actual_taint, all_expected_taint_ids);
        assert_eq!(actual_dependencies, all_expected_dependencies_ids);
    }

    fn test_input(
        builder: FlowBuilder<'_>,
        location: LocationId,
        expected_taint: BTreeMap<&str, BTreeSet<&str>>,
        expected_dependencies: BTreeMap<&str, BTreeMap<&str, StructOrTuple>>,
    ) {
        test_input_with_unnamed_ir_nodes(
            builder,
            location,
            expected_taint,
            BTreeMap::new(),
            expected_dependencies,
            BTreeMap::new(),
        );
    }

    fn test_input_partitionable(
        builder: FlowBuilder<'_>,
        location: LocationId,
        expected_partitionings: Option<Vec<BTreeMap<&str, StructOrTupleIndex>>>,
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
        let partitioning = partitioning_analysis(&mut ir, &location, &cycle_data);

        if expected_partitionings.is_none() {
            assert!(partitioning.is_none());
            return;
        }

        // Convert names to IDs
        let name_to_id = name_to_id_map(&mut ir);
        let expected_partitionings_ids = expected_partitionings.map(|vec| {
            vec.into_iter()
                .map(|map| {
                    map.into_iter()
                        .map(|(k, v)| (*name_to_id.get(k).unwrap(), v.clone()))
                        .collect::<BTreeMap<_, _>>()
                })
                .collect::<Vec<_>>()
        });

        assert_eq!(partitioning.unwrap().0, expected_partitionings_ids.unwrap());
    }

    #[test]
    fn test_map() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .ir_node_named("network")
            .values()
            .ir_node_named("the map following network")
            .map(q!(|(a, b)| (b, a + 2)))
            .ir_node_named("the operator being tested")
            .for_each(q!(|(b, a2)| {
                println!("b: {}, a+2: {}", b, a2);
            }));

        let expected_taint = BTreeMap::from([
            ("the map following network", BTreeSet::from(["network"])),
            ("the operator being tested", BTreeSet::from(["network"])),
        ]);

        let mut implicit_map_dependencies = StructOrTuple::default();
        implicit_map_dependencies.add_dependency(&vec![], vec!["1".to_string()]);
        let mut map_expected_dependencies = StructOrTuple::default();
        map_expected_dependencies.add_dependency(
            &vec!["0".to_string()],
            vec!["1".to_string(), "1".to_string()],
        );

        let expected_dependencies = BTreeMap::from([
            ("network", BTreeMap::new()),
            (
                "the map following network",
                BTreeMap::from([("network", implicit_map_dependencies)]),
            ),
            (
                "the operator being tested",
                BTreeMap::from([("network", map_expected_dependencies)]),
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
    fn test_map_partitionable() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .values()
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
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .ir_node_named("network")
            .values()
            .ir_node_named("map after network")
            .map(q!(|(a, b)| (b.1, a, b.0 - a)))
            .ir_node_named("map 1")
            .map(q!(|(b1, _a, b0a)| (b0a, b1.0)))
            .ir_node_named("map 2")
            .for_each(q!(|(b0a, b10)| {
                println!("b.0 - a: {}, b.1.0: {}", b0a, b10);
            }));

        let expected_taint = BTreeMap::from([
            ("map after network", BTreeSet::from(["network"])),
            ("map 1", BTreeSet::from(["network"])),
            ("map 2", BTreeSet::from(["network"])),
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
            ("network", BTreeMap::new()),
            (
                "map after network",
                BTreeMap::from([("network", implicit_map_dependencies)]),
            ),
            (
                "map 1",
                BTreeMap::from([("network", map1_expected_dependencies)]),
            ),
            (
                "map 2",
                BTreeMap::from([("network", map2_expected_dependencies)]),
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
    fn test_filter_map() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .ir_node_named("network")
            .values()
            .ir_node_named("map after network")
            .filter_map(q!(|(a, b)| { if a > 1 { Some((b, a + 2)) } else { None } }))
            .ir_node_named("operator")
            .for_each(q!(|(b, a2)| {
                println!("b: {}, a+2: {}", b, a2);
            }));

        let expected_taint = BTreeMap::from([
            ("map after network", BTreeSet::from(["network"])),
            ("operator", BTreeSet::from(["network"])),
        ]);

        let mut implicit_map_dependencies = StructOrTuple::default();
        implicit_map_dependencies.add_dependency(&vec![], vec!["1".to_string()]);
        let mut map_expected_dependencies = StructOrTuple::default();
        map_expected_dependencies.add_dependency(
            &vec!["0".to_string()],
            vec!["1".to_string(), "1".to_string()],
        );

        let expected_dependencies = BTreeMap::from([
            ("network", BTreeMap::new()),
            (
                "map after network",
                BTreeMap::from([("network", implicit_map_dependencies)]),
            ),
            (
                "operator",
                BTreeMap::from([("network", map_expected_dependencies)]),
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
    fn test_filter_map_remove_none() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .ir_node_named("network")
            .values()
            .ir_node_named("map after network")
            .filter_map(q!(|(a, b)| {
                if a > 1 {
                    Some((None, a + 2))
                } else if a < 1 {
                    Some((Some(b), a + 2))
                } else {
                    None
                }
            }))
            .ir_node_named("operator")
            .for_each(q!(|(none, a2)| {
                println!("None: {:?}, a+2: {}", none, a2);
            }));

        let expected_taint = BTreeMap::from([
            ("map after network", BTreeSet::from(["network"])),
            ("operator", BTreeSet::from(["network"])),
        ]);

        let mut implicit_map_dependencies = StructOrTuple::default();
        implicit_map_dependencies.add_dependency(&vec![], vec!["1".to_string()]);

        let expected_dependencies = BTreeMap::from([
            ("network", BTreeMap::new()),
            (
                "map after network",
                BTreeMap::from([("network", implicit_map_dependencies)]),
            ),
            ("operator", BTreeMap::new()),
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
        cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .ir_node_named("network")
            .values()
            .ir_node_named("map after network")
            .batch(&cluster2.tick(), nondet!(/** test */))
            .delta()
            .ir_node_named("operator")
            .all_ticks()
            .for_each(q!(|(a, b)| {
                println!("a: {}, b: {}", a, b);
            }));

        let expected_taint = BTreeMap::from([
            ("map after network", BTreeSet::from(["network"])),
            ("operator", BTreeSet::from(["network"])),
        ]);

        let mut implicit_map_dependencies = StructOrTuple::default();
        implicit_map_dependencies.add_dependency(&vec![], vec!["1".to_string()]);

        let expected_dependencies = BTreeMap::from([
            ("network", BTreeMap::new()),
            (
                "map after network",
                BTreeMap::from([("network", implicit_map_dependencies.clone())]),
            ),
            (
                "operator",
                BTreeMap::from([("network", implicit_map_dependencies)]),
            ), // No dependency changes from parent
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
        cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .values()
            .batch(&cluster2.tick(), nondet!(/** test */))
            .delta()
            .all_ticks()
            .for_each(q!(|(a, b)| {
                println!("a: {}, b: {}", a, b);
            }));

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
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .ir_node_named("network")
            .values()
            .ir_node_named("map after network");
        let stream1 = input
            .clone()
            .ir_node_named("teed input 1")
            .map(q!(|(a, b)| (b, a + 2)))
            .ir_node_named("map 1");
        let stream2 = input
            .ir_node_named("teed input 2")
            .map(q!(|(a, b)| ((b.1, b.1), a + 3)))
            .ir_node_named("map 2");
        let tick = cluster2.tick();
        stream2
            .batch(&tick, nondet!(/** test */))
            .chain(stream1.batch(&tick, nondet!(/** test */)))
            .ir_node_named("chain")
            .all_ticks()
            .for_each(q!(|((x, b1), y)| {
                println!("x: {}, b.1: {}, y: {}", x, b1, y);
            }));

        let expected_taint = BTreeMap::from([
            ("map after network", BTreeSet::from(["network"])),
            ("teed input 2", BTreeSet::from(["network"])),
            ("map 2", BTreeSet::from(["network"])),
            ("teed input 1", BTreeSet::from(["network"])),
            ("map 1", BTreeSet::from(["network"])),
            ("chain", BTreeSet::from(["network"])),
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
            ("network", BTreeMap::new()),
            (
                "map after network",
                BTreeMap::from([("network", implicit_map_dependencies.clone())]),
            ),
            (
                "teed input 2",
                BTreeMap::from([("network", implicit_map_dependencies.clone())]),
            ),
            (
                "map 2",
                BTreeMap::from([("network", stream2_map_dependencies)]),
            ),
            (
                "teed input 1",
                BTreeMap::from([("network", implicit_map_dependencies)]),
            ),
            (
                "map 1",
                BTreeMap::from([("network", stream1_map_dependencies)]),
            ),
            ("chain", BTreeMap::from([("network", chain_dependencies)])),
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
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .values();
        let stream1 = input.clone().map(q!(|(a, b)| (b, a + 2)));
        let stream2 = input.map(q!(|(a, b)| ((b.1, b.1), a + 3)));
        let tick = cluster2.tick();
        stream2
            .batch(&tick, nondet!(/** test */))
            .chain(stream1.batch(&tick, nondet!(/** test */)))
            .all_ticks()
            .for_each(q!(|((x, b1), y)| {
                println!("x: {}, b.1: {}, y: {}", x, b1, y);
            }));

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
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .ir_node_named("network")
            .values()
            .ir_node_named("map after network");
        let stream1 = input
            .clone()
            .ir_node_named("teed input 1")
            .map(q!(|(a, b)| (b, a + 2)))
            .ir_node_named("map 1");
        let stream2 = input
            .ir_node_named("teed input 2")
            .map(q!(|(a, b)| ((b.1, b.1), a + 3)))
            .ir_node_named("map 2");
        let tick = cluster2.tick();
        stream2
            .batch(&tick, nondet!(/** test */))
            .cross_product(stream1.batch(&tick, nondet!(/** test */)))
            .ir_node_named("cross product")
            .all_ticks()
            .for_each(q!(|(((b1, b1_again), a3), (b, a2))| {
                println!("((({}, {}), {}), ({:?}, {}))", b1, b1_again, a3, b, a2);
            }));

        let expected_taint = BTreeMap::from([
            ("map after network", BTreeSet::from(["network"])),
            ("teed input 2", BTreeSet::from(["network"])),
            ("map 2", BTreeSet::from(["network"])),
            ("teed input 1", BTreeSet::from(["network"])),
            ("map 1", BTreeSet::from(["network"])),
            ("cross product", BTreeSet::from(["network"])),
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
            ("network", BTreeMap::new()),
            (
                "map after network",
                BTreeMap::from([("network", implicit_map_dependencies.clone())]),
            ),
            (
                "teed input 2",
                BTreeMap::from([("network", implicit_map_dependencies.clone())]),
            ),
            (
                "map 2",
                BTreeMap::from([("network", stream2_map_dependencies)]),
            ),
            (
                "teed input 1",
                BTreeMap::from([("network", implicit_map_dependencies)]),
            ),
            (
                "map 1",
                BTreeMap::from([("network", stream1_map_dependencies)]),
            ),
            (
                "cross product",
                BTreeMap::from([("network", cross_product_dependencies)]),
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
    fn test_cross_product_partitionable() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        let input = cluster1
            .source_iter(q!([(1, (2, 3))]))
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .values();
        let stream1 = input.clone().map(q!(|(a, b)| (b, a + 2)));
        let stream2 = input.map(q!(|(a, b)| ((b.1, b.1), a + 3)));
        let tick = cluster2.tick();
        stream2
            .batch(&tick, nondet!(/** test */))
            .cross_product(stream1.batch(&tick, nondet!(/** test */)))
            .all_ticks()
            .for_each(q!(|(((b1, b1_again), a3), (b, a2))| {
                println!("((({}, {}), {}), ({:?}, {}))", b1, b1_again, a3, b, a2);
            }));

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
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .ir_node_named("network")
            .values()
            .ir_node_named("map after network");
        let stream1 = input
            .clone()
            .ir_node_named("teed input 1")
            .map(q!(|(a, b)| (b, a)))
            .ir_node_named("map 1");
        let stream2 = input
            .ir_node_named("teed input 2")
            .map(q!(|(a, b)| ((b.1, b.1), a + 3)))
            .ir_node_named("map 2");
        let tick = cluster2.tick();
        stream2
            .batch(&tick, nondet!(/** test */))
            .join(stream1.batch(&tick, nondet!(/** test */)))
            .ir_node_named("join")
            .all_ticks()
            .for_each(q!(|((b1, b1_again), (a3, a))| {
                println!("(({}, {}), {}, {})", b1, b1_again, a3, a);
            }));

        let expected_taint = BTreeMap::from([
            ("map after network", BTreeSet::from(["network"])),
            ("teed input 2", BTreeSet::from(["network"])),
            ("map 2", BTreeSet::from(["network"])),
            ("teed input 1", BTreeSet::from(["network"])),
            ("map 1", BTreeSet::from(["network"])),
            ("join", BTreeSet::from(["network"])),
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
            ("network", BTreeMap::new()),
            (
                "map after network",
                BTreeMap::from([("network", implicit_map_dependencies.clone())]),
            ),
            (
                "teed input 2",
                BTreeMap::from([("network", implicit_map_dependencies.clone())]),
            ),
            (
                "map 2",
                BTreeMap::from([("network", stream2_map_dependencies)]),
            ),
            (
                "teed input 1",
                BTreeMap::from([("network", implicit_map_dependencies)]),
            ),
            (
                "map 1",
                BTreeMap::from([("network", stream1_map_dependencies)]),
            ),
            ("join", BTreeMap::from([("network", join_dependencies)])),
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
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .ir_node_named("network")
            .values();
        let stream1 = input.clone().map(q!(|(a, b)| (b, a)));
        let stream2 = input.map(q!(|(a, b)| ((b.1, b.1), a + 3)));
        let tick = cluster2.tick();
        stream2
            .batch(&tick, nondet!(/** test */))
            .join(stream1.batch(&tick, nondet!(/** test */)))
            .all_ticks()
            .for_each(q!(|((b1, b1_again), (a3, a))| {
                println!("(({}, {}), {}, {})", b1, b1_again, a3, a);
            }));

        // Can either partition on b.0 or b.1, since the join is only successful when both b.0=b.1 or b.1=b.1
        let expected_partitionings = Some(vec![
            BTreeMap::from([(
                "network",
                vec!["1".to_string(), "1".to_string(), "0".to_string()],
            )]),
            BTreeMap::from([(
                "network",
                vec!["1".to_string(), "1".to_string(), "1".to_string()],
            )]),
        ]);
        test_input_partitionable(builder, cluster2.id(), expected_partitionings);
    }

    #[test]
    fn test_enumerate() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .ir_node_named("network")
            // Batching is unnecessary, but otherwise enumerate will be implicitly wrapped with a Persist (removed with persist_pullup)
            .batch(&cluster1.tick(), nondet!(/** test */))
            .values()
            .ir_node_named("map after network")
            .assume_ordering(nondet!(/** test */))
            .enumerate()
            .ir_node_named("enumerate")
            .all_ticks()
            .for_each(q!(|(i, (a, b))| {
                println!("i: {}, a: {}, b: {}", i, a, b);
            }));

        let expected_taint = BTreeMap::from([
            ("map after network", BTreeSet::from(["network"])),
            ("enumerate", BTreeSet::from(["network"])),
        ]);

        let mut implicit_map_dependencies = StructOrTuple::default();
        implicit_map_dependencies.add_dependency(&vec![], vec!["1".to_string()]);
        let mut enumerate_expected_dependencies = StructOrTuple::default();
        enumerate_expected_dependencies
            .add_dependency(&vec!["1".to_string()], vec!["1".to_string()]);

        let expected_dependencies = BTreeMap::from([
            ("network", BTreeMap::new()),
            (
                "map after network",
                BTreeMap::from([("network", implicit_map_dependencies)]),
            ),
            (
                "enumerate",
                BTreeMap::from([("network", enumerate_expected_dependencies)]),
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
    fn test_enumerate_partitionable() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .values()
            .assume_ordering(nondet!(/** test */))
            .enumerate()
            .for_each(q!(|(i, (a, b))| {
                println!("i: {}, a: {}, b: {}", i, a, b);
            }));

        let expected_partitionings = None;
        test_input_partitionable(builder, cluster2.id(), expected_partitionings);
    }

    #[test]
    fn test_reduce_keyed() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .ir_node_named("network")
            .values()
            .ir_node_named("map after network")
            .batch(&cluster2.tick(), nondet!(/** test */))
            .into_keyed()
            .reduce_commutative(q!(|acc, b| *acc += b))
            .ir_node_named("reduce keyed")
            .entries()
            .all_ticks()
            .for_each(q!(|(a, b_sum)| {
                println!("a: {}, b_sum: {}", a, b_sum);
            }));

        let expected_taint = BTreeMap::from([
            ("map after network", BTreeSet::from(["network"])),
            ("reduce keyed", BTreeSet::from(["network"])),
        ]);

        let mut implicit_map_dependencies = StructOrTuple::default();
        implicit_map_dependencies.add_dependency(&vec![], vec!["1".to_string()]);
        let mut reduce_keyed_expected_dependencies = StructOrTuple::default();
        reduce_keyed_expected_dependencies.add_dependency(
            &vec!["0".to_string()],
            vec!["1".to_string(), "0".to_string()],
        );

        let expected_dependencies = BTreeMap::from([
            ("network", BTreeMap::new()),
            (
                "map after network",
                BTreeMap::from([("network", implicit_map_dependencies)]),
            ),
            (
                "reduce keyed",
                BTreeMap::from([("network", reduce_keyed_expected_dependencies)]),
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
    fn test_reduce_keyed_partitionable() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .ir_node_named("network")
            .values()
            .batch(&cluster2.tick(), nondet!(/** test */))
            .into_keyed()
            .reduce_commutative(q!(|acc, b| *acc += b))
            .entries()
            .all_ticks()
            .for_each(q!(|(a, b_sum)| {
                println!("a: {}, b_sum: {}", a, b_sum);
            }));

        let expected_partitionings = Some(vec![BTreeMap::from([(
            "network",
            vec!["1".to_string(), "0".to_string()],
        )])]);
        test_input_partitionable(builder, cluster2.id(), expected_partitionings);
    }

    #[test]
    fn test_reduce() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .ir_node_named("network")
            .values()
            .ir_node_named("map after network")
            .batch(&cluster2.tick(), nondet!(/** test */))
            .reduce_commutative(q!(|(acc_a, acc_b), (a, b)| {
                *acc_a += a;
                *acc_b += b;
            }))
            .ir_node_named("reduce")
            .all_ticks()
            .for_each(q!(|(a_sum, b_sum)| {
                println!("a_sum: {}, b_sum: {}", a_sum, b_sum);
            }));

        let expected_taint = BTreeMap::from([
            ("map after network", BTreeSet::from(["network"])),
            ("reduce", BTreeSet::from(["network"])),
        ]);

        let mut implicit_map_dependencies = StructOrTuple::default();
        implicit_map_dependencies.add_dependency(&vec![], vec!["1".to_string()]);
        let expected_dependencies = BTreeMap::from([
            ("network", BTreeMap::new()),
            (
                "map after network",
                BTreeMap::from([("network", implicit_map_dependencies)]),
            ),
            ("reduce", BTreeMap::new()),
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
        cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .values()
            .batch(&cluster2.tick(), nondet!(/** test */))
            .reduce_commutative(q!(|(acc_a, acc_b), (a, b)| {
                *acc_a += a;
                *acc_b += b;
            }))
            .all_ticks()
            .for_each(q!(|(a_sum, b_sum)| {
                println!("a_sum: {}, b_sum: {}", a_sum, b_sum);
            }));

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
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .ir_node_named("network")
            .values()
            .ir_node_named("map after network");
        let cluster2_tick = cluster2.tick();
        let (complete_cycle, cycle) =
            cluster2_tick.cycle::<Stream<(usize, usize), _, Bounded, NoOrder>>();
        let prev_tick_input = cycle
            .clone()
            .ir_node_named("teed cycle 1")
            .filter(q!(|(a, _b)| *a > 2))
            .ir_node_named("filter")
            .map(q!(|(a, b)| (a, b + 2)))
            .ir_node_named("map");
        complete_cycle.complete_next_tick(
            prev_tick_input
                .chain(input.batch(&cluster2_tick, nondet!(/** test */)))
                .ir_node_named("chain"),
        );

        cycle
            .ir_node_named("teed cycle 2")
            .all_ticks()
            .for_each(q!(|(a, b)| {
                println!("a: {}, b: {}", a, b);
            }));

        let expected_taint = BTreeMap::from([
            ("teed cycle 1", BTreeSet::from(["network"])),
            ("filter", BTreeSet::from(["network"])),
            ("map", BTreeSet::from(["network"])),
            ("map after network", BTreeSet::from(["network"])),
            ("chain", BTreeSet::from(["network"])),
            ("teed cycle 2", BTreeSet::from(["network"])),
        ]);
        // IR nodes aren't exposed, so we'll have to find them through relative positioning
        let unnamed_expected_taint = BTreeMap::from([
            (("teed cycle 1", -1), BTreeSet::from(["network"])), // CycleSource
            (("teed cycle 2", -1), BTreeSet::from(["network"])), // DeferTick
        ]);

        let mut implicit_map_dependencies = StructOrTuple::default();
        implicit_map_dependencies.add_dependency(&vec![], vec!["1".to_string()]);
        let mut cycle_dependencies = StructOrTuple::default();
        cycle_dependencies.add_dependency(
            &vec!["0".to_string()],
            vec!["1".to_string(), "0".to_string()],
        );

        let expected_dependencies = BTreeMap::from([
            (
                "teed cycle 1",
                BTreeMap::from([("network", cycle_dependencies.clone())]),
            ),
            (
                "filter",
                BTreeMap::from([("network", cycle_dependencies.clone())]),
            ),
            (
                "map",
                BTreeMap::from([("network", cycle_dependencies.clone())]),
            ),
            ("network", BTreeMap::new()),
            (
                "map after network",
                BTreeMap::from([("network", implicit_map_dependencies)]),
            ),
            (
                "chain",
                BTreeMap::from([("network", cycle_dependencies.clone())]),
            ),
            (
                "teed cycle 2",
                BTreeMap::from([("network", cycle_dependencies.clone())]),
            ),
        ]);

        let unnamed_expected_dependencies = BTreeMap::from([
            (
                ("teed cycle 1", -1),
                BTreeMap::from([("network", cycle_dependencies.clone())]),
            ), // CycleSource
            (
                ("teed cycle 2", -1),
                BTreeMap::from([("network", cycle_dependencies)]),
            ), // DeferTick
        ]);

        test_input_with_unnamed_ir_nodes(
            builder,
            cluster2.id(),
            expected_taint,
            unnamed_expected_taint,
            expected_dependencies,
            unnamed_expected_dependencies,
        );
    }

    #[test]
    fn test_cycle_partitionable() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        let input = cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .values();
        let cluster2_tick = cluster2.tick();
        let (complete_cycle, cycle) =
            cluster2_tick.cycle::<Stream<(usize, usize), _, Bounded, NoOrder>>();
        let prev_tick_input = cycle
            .clone()
            .filter(q!(|(a, _b)| *a > 2))
            .map(q!(|(a, b)| (a, b + 2)));
        complete_cycle.complete_next_tick(
            prev_tick_input.chain(input.batch(&cluster2_tick, nondet!(/** test */))),
        );

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
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .ir_node_named("network")
            .values()
            .ir_node_named("map after network");
        let cluster2_tick = cluster2.tick();
        let (complete_cycle1, cycle1) =
            cluster2_tick.cycle::<Stream<(usize, usize), _, Bounded, NoOrder>>();
        let (complete_cycle2, cycle2) =
            cluster2_tick.cycle::<Stream<(usize, usize), _, Bounded, NoOrder>>();
        let chained = {
            cycle1.join(input.batch(&cluster2_tick, nondet!(/** test */)))
                .ir_node_named("join")
                .map(q!(|(_, (b1,b2))| (b1,b2))) // Both values are influenced by the join with cycle2_out
                .ir_node_named("map (x,(a,b)) to (a,b)")
                .chain(cycle2)
        };
        complete_cycle1.complete_next_tick(chained.clone().ir_node_named("teed chain 1"));
        let cycle2_out = chained
            .ir_node_named("teed chain 2")
            .map(q!(|(_a, b)| (b, b)));
        complete_cycle2.complete_next_tick(
            cycle2_out
                .clone()
                .ir_node_named("teed map (a,b) to (b,b) 1"),
        );
        cycle2_out
            .ir_node_named("teed map (a,b) to (b,b) 2")
            .all_ticks()
            .for_each(q!(|(b, _)| {
                println!("b: {}", b);
            }));

        let expected_taint = BTreeMap::from([
            ("map after network", BTreeSet::from(["network"])),
            ("join", BTreeSet::from(["network"])),
            ("map (x,(a,b)) to (a,b)", BTreeSet::from(["network"])), // map (x,(a,b)) to (a,b)
            ("teed chain 1", BTreeSet::from(["network"])),           // Tee(chain)
            ("teed chain 2", BTreeSet::from(["network"])),           // Tee(chain)
            ("teed map (a,b) to (b,b) 1", BTreeSet::from(["network"])), // Tee(map)
            ("teed map (a,b) to (b,b) 2", BTreeSet::from(["network"])), // Tee(map)
        ]);
        let unnamed_expected_taint = BTreeMap::from([
            (("network", -8), BTreeSet::from(["network"])), /* CycleSource(cycle1), parent = DeferTick(cycle1) */
            (("map (x,(a,b)) to (a,b)", 1), BTreeSet::from(["network"])), // CycleSource(cycle2)
            (("teed chain 1", -1), BTreeSet::from(["network"])), // Chain
            (("teed chain 1", 1), BTreeSet::from(["network"])), // DeferTick(cycle1)
            (
                ("teed map (a,b) to (b,b) 1", -1),
                BTreeSet::from(["network"]),
            ), // map (a,b) to (b,b)
            (
                ("teed map (a,b) to (b,b) 1", 1),
                BTreeSet::from(["network"]),
            ), // DeferTick(cycle2)
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
            ("network", BTreeMap::new()),
            (
                "map after network",
                BTreeMap::from([("network", implicit_map_dependencies)]),
            ),
            ("join", BTreeMap::from([("network", join_dependencies)])),
            (
                "map (x,(a,b)) to (a,b)",
                BTreeMap::from([("network", other_dependencies.clone())]),
            ),
            (
                "teed chain 1",
                BTreeMap::from([("network", other_dependencies.clone())]),
            ), // Tee(chain)
            (
                "teed chain 2",
                BTreeMap::from([("network", other_dependencies.clone())]),
            ), // Tee(chain)
            (
                "teed map (a,b) to (b,b) 1",
                BTreeMap::from([("network", other_dependencies.clone())]),
            ), // Tee(map)
            (
                "teed map (a,b) to (b,b) 2",
                BTreeMap::from([("network", other_dependencies.clone())]),
            ), // Tee(map)
        ]);
        let unnamed_expected_dependencies = BTreeMap::from([
            (
                ("network", -8),
                BTreeMap::from([("network", other_dependencies.clone())]),
            ), // CycleSource(cycle1), parent = DeferTick(cycle1)
            (
                ("map (x,(a,b)) to (a,b)", 1),
                BTreeMap::from([("network", other_dependencies.clone())]),
            ), // CycleSource(cycle2)
            (
                ("teed chain 1", -1),
                BTreeMap::from([("network", other_dependencies.clone())]),
            ), // Chain
            (
                ("teed chain 1", 1),
                BTreeMap::from([("network", other_dependencies.clone())]),
            ), // DeferTick(cycle1)
            (
                ("teed map (a,b) to (b,b) 1", -1),
                BTreeMap::from([("network", other_dependencies.clone())]),
            ), // map (a,b) to (b,b)
            (
                ("teed map (a,b) to (b,b) 1", 1),
                BTreeMap::from([("network", other_dependencies)]),
            ), // DeferTick(cycle2)
        ]);

        test_input_with_unnamed_ir_nodes(
            builder,
            cluster2.id(),
            expected_taint,
            unnamed_expected_taint,
            expected_dependencies,
            unnamed_expected_dependencies,
        );
    }

    #[test]
    fn test_nested_cycle_partitionable() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        let input = cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .ir_node_named("network")
            .values();
        let cluster2_tick = cluster2.tick();
        let (complete_cycle1, cycle1) =
            cluster2_tick.cycle::<Stream<(usize, usize), _, Bounded, NoOrder>>();
        let (complete_cycle2, cycle2) =
            cluster2_tick.cycle::<Stream<(usize, usize), _, Bounded, NoOrder>>();
        let chained = {
            cycle1.join(input.batch(&cluster2_tick, nondet!(/** test */)))
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
            "network",
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
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .ir_node_named("network")
            .values()
            .ir_node_named("map after network");
        let tick = cluster2.tick();
        let stream1 = input.map(q!(|(a, b)| (b, a + 2))).ir_node_named("map");
        let stream2 = cluster2.source_iter(q!([(3, 4)]));
        stream2
            .batch(&tick, nondet!(/** test */))
            .chain(stream1.batch(&tick, nondet!(/** test */)))
            .ir_node_named("chain")
            .all_ticks()
            .for_each(q!(|_| {
                println!("No dependencies");
            }));

        let expected_taint = BTreeMap::from([
            ("map after network", BTreeSet::from(["network"])),
            ("map", BTreeSet::from(["network"])),
            ("chain", BTreeSet::from(["network"])),
        ]);
        let unnamed_expected_taint = BTreeMap::from([]);

        let mut implicit_map_dependencies = StructOrTuple::default();
        implicit_map_dependencies.add_dependency(&vec![], vec!["1".to_string()]);
        let mut map_dependencies = StructOrTuple::default();
        map_dependencies.add_dependency(
            &vec!["0".to_string()],
            vec!["1".to_string(), "1".to_string()],
        );

        let expected_dependencies = BTreeMap::from([
            ("network", BTreeMap::new()),
            (
                "map after network",
                BTreeMap::from([("network", implicit_map_dependencies)]),
            ),
            (
                "map",
                BTreeMap::from([("network", map_dependencies.clone())]),
            ),
            ("chain", BTreeMap::from([("network", map_dependencies)])),
        ]);
        let unnamed_expected_dependencies = BTreeMap::from([
            (("network", -8), BTreeMap::from([])), // Source
        ]);

        test_input_with_unnamed_ir_nodes(
            builder,
            cluster2.id(),
            expected_taint,
            unnamed_expected_taint,
            expected_dependencies,
            unnamed_expected_dependencies,
        );
    }

    #[test]
    fn test_source_iter_partitionable() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        let input = cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .values();
        let tick = cluster2.tick();
        let stream1 = input.map(q!(|(a, b)| (b, a + 2)));
        let stream2 = cluster2.source_iter(q!([(3, 4)]));
        stream2
            .batch(&tick, nondet!(/** test */))
            .chain(stream1.batch(&tick, nondet!(/** test */)))
            .all_ticks()
            .for_each(q!(|_| {
                println!("No dependencies");
            }));

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
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .ir_node_named("input1")
            .values()
            .ir_node_named("map after input1");
        let input2 = cluster1
            .source_iter(q!([(3, 4)]))
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .ir_node_named("input2")
            .values()
            .ir_node_named("map after input2");
        let tick = cluster2.tick();
        let stream1 = input1
            .map(q!(|(a, b)| (a * 2, b)))
            .batch(&tick, nondet!(/** test */));
        let stream2 = input2
            .map(q!(|(a, b)| (-a, b)))
            .batch(&tick, nondet!(/** test */));
        stream2
            .clone()
            .ir_node_named("teed map2 1")
            .chain(stream1.clone().ir_node_named("teed map1 1"))
            .ir_node_named("chain")
            .all_ticks()
            .for_each(q!(|_| {
                println!("Dependent on both input1.b and input2.b");
            }));
        stream2
            .ir_node_named("teed map2 2")
            .join(stream1.ir_node_named("teed map1 2"))
            .ir_node_named("join")
            .all_ticks()
            .for_each(q!(|(_, (b1, b2))| {
                println!("b from input 1: {}, b from input 2: {}", b1, b2);
            }));

        let expected_taint = BTreeMap::from([
            ("map after input2", BTreeSet::from(["input2"])),
            ("teed map2 1", BTreeSet::from(["input2"])),
            ("map after input1", BTreeSet::from(["input1"])),
            ("teed map1 1", BTreeSet::from(["input1"])),
            ("chain", BTreeSet::from(["input1", "input2"])),
            ("teed map2 2", BTreeSet::from(["input2"])),
            ("teed map1 2", BTreeSet::from(["input1"])),
            ("join", BTreeSet::from(["input1", "input2"])),
        ]);
        let unnamed_expected_taint = BTreeMap::from([
            (("teed map2 1", -1), BTreeSet::from(["input2"])), // map2
            (("teed map1 1", -1), BTreeSet::from(["input1"])), // map1
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
            ("input2", BTreeMap::new()),
            (
                "map after input2",
                BTreeMap::from([("input2", implicit_map_dependencies.clone())]),
            ),
            (
                "teed map2 1",
                BTreeMap::from([("input2", input_map_dependencies.clone())]),
            ),
            ("input1", BTreeMap::new()),
            (
                "map after input1",
                BTreeMap::from([("input1", implicit_map_dependencies)]),
            ),
            (
                "teed map1 1",
                BTreeMap::from([("input1", input_map_dependencies.clone())]),
            ),
            (
                "chain",
                BTreeMap::from([
                    ("input1", input_map_dependencies.clone()),
                    ("input2", input_map_dependencies.clone()),
                ]),
            ),
            (
                "teed map2 2",
                BTreeMap::from([("input2", input_map_dependencies.clone())]),
            ),
            (
                "teed map1 2",
                BTreeMap::from([("input1", input_map_dependencies.clone())]),
            ),
            (
                "join",
                BTreeMap::from([
                    ("input1", join_input1_dependencies),
                    ("input2", join_input2_dependencies),
                ]),
            ),
        ]);
        let unnamed_expected_dependencies = BTreeMap::from([
            (
                ("teed map2 1", -1),
                BTreeMap::from([("input2", input_map_dependencies.clone())]),
            ), // map2
            (
                ("teed map1 1", -1),
                BTreeMap::from([("input1", input_map_dependencies)]),
            ), // map1
        ]);

        test_input_with_unnamed_ir_nodes(
            builder,
            cluster2.id(),
            expected_taint,
            unnamed_expected_taint,
            expected_dependencies,
            unnamed_expected_dependencies,
        );
    }

    #[test]
    fn test_multiple_inputs_partitionable() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        let input1 = cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .ir_node_named("input1")
            .values();
        let input2 = cluster1
            .source_iter(q!([(3, 4)]))
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .ir_node_named("input2")
            .values();
        let tick = cluster2.tick();
        let stream1 = input1
            .map(q!(|(a, b)| (b, a * 2)))
            .batch(&tick, nondet!(/** test */));
        let stream2 = input2
            .map(q!(|(a, b)| (b, -a)))
            .batch(&tick, nondet!(/** test */));
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

        let expected_partitioning = Some(vec![BTreeMap::from([
            ("input2", vec!["1".to_string(), "1".to_string()]),
            ("input1", vec!["1".to_string(), "1".to_string()]),
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
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .ir_node_named("input1")
            .values()
            .ir_node_named("map after input1");
        let input2 = cluster1
            .source_iter(q!([(3, 4)]))
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .ir_node_named("input2")
            .values()
            .ir_node_named("map after input2");
        let tick = cluster2.tick();
        input1
            .batch(&tick, nondet!(/** test */))
            .filter_not_in(input2.batch(&tick, nondet!(/** test */)))
            .ir_node_named("difference")
            .all_ticks()
            .for_each(q!(|(a, b)| {
                println!("a: {}, b: {}", a, b);
            }));

        let expected_taint = BTreeMap::from([
            ("map after input1", BTreeSet::from(["input1"])),
            ("map after input2", BTreeSet::from(["input2"])),
            ("difference", BTreeSet::from(["input1"])), // Isn't tainted by anti-joined parent
        ]);

        let mut implicit_map_dependencies = StructOrTuple::default();
        implicit_map_dependencies.add_dependency(&vec![], vec!["1".to_string()]);

        let expected_dependencies = BTreeMap::from([
            ("input1", BTreeMap::new()),
            (
                "map after input1",
                BTreeMap::from([("input1", implicit_map_dependencies.clone())]),
            ),
            ("input2", BTreeMap::new()),
            (
                "map after input2",
                BTreeMap::from([("input2", implicit_map_dependencies.clone())]),
            ),
            (
                "difference",
                BTreeMap::from([("input1", implicit_map_dependencies)]),
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
    fn test_difference_partitionable() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        let input1 = cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .ir_node_named("input1")
            .values();
        let input2 = cluster1
            .source_iter(q!([(3, 4)]))
            .broadcast_bincode(&cluster2, nondet!(/** test */))
            .ir_node_named("input2")
            .values();
        let tick = cluster2.tick();
        input1
            .batch(&tick, nondet!(/** test */))
            .filter_not_in(input2.batch(&tick, nondet!(/** test */)))
            .all_ticks()
            .for_each(q!(|(a, b)| {
                println!("a: {}, b: {}", a, b);
            }));

        let expected_partitionings = Some(vec![BTreeMap::from([
            ("input1", vec!["1".to_string()]),
            ("input2", vec!["1".to_string()]),
        ])]);
        test_input_partitionable(builder, cluster2.id(), expected_partitionings);
    }
}
