use std::{collections::{HashMap, HashSet}, hash::{DefaultHasher, Hasher, Hash}};
use syn::visit::Visit;

use hydro_lang::{ir::{traverse_dfir, HydroLeaf, HydroNode}, location::LocationId};

use crate::{parse_results::{get_network_type, NetworkType}, partition_syn_analysis::StructOrTuple, rewrites::relevant_inputs, partition_syn_analysis::AnalyzeClosure};

// Find all inputs of a node
struct InputMetadata {
    // Const fields
    cluster_to_partition: LocationId,
    // Variables
    inputs: HashSet<usize>, // op_ids of cluster inputs.
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

fn input_analysis(ir: &mut [HydroLeaf], cluster_to_partition: LocationId) -> HashSet<usize> {
    let mut input_metadata = InputMetadata {
        cluster_to_partition,
        inputs: HashSet::new(),
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
    pub inputs: HashSet<usize>,
    // Variables
    pub input_taint: HashMap<usize, HashSet<usize>>, // op_id -> set of input op_ids that taint this node
    pub input_dependencies: HashMap<usize, HashMap<usize, StructOrTuple>>, // op_id -> (input op_id -> index of input in output)
    pub syn_analysis: HashMap<usize, StructOrTuple>, // Cached results for analyzing f for each operator
}

impl Hash for InputDependencyMetadata {
    // Only consider input_taint and input_dependencies
    fn hash<H: Hasher>(&self, state: &mut H) {
        format!("{:#?}", self.input_taint).hash(state);
        format!("{:#?}", self.input_dependencies).hash(state);
    }
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

    println!("Analyzing node {:?} with next_stmt_id {}", node.print_root(), next_stmt_id);

    let parent_ids = match node {
        HydroNode::CycleSource { .. } => {
            // For CycleSource, its input is its CycleSink's input. Note: assume the CycleSink is on the same cluster
            vec![*cycle_source_to_sink_input.get(next_stmt_id).unwrap()]
        }
        HydroNode::Tee { inner, .. } => {
            vec![inner.0.borrow().metadata().id.unwrap()]
        }
        _ => relevant_inputs(
            node.input_metadata(),
            &metadata.cluster_to_partition,
        ),
    };

    let InputDependencyMetadata { inputs, input_taint, input_dependencies, syn_analysis, .. } = metadata;

    // Calculate input taints, find parent input dependencies
    let mut parent_input_dependencies: HashMap<usize, HashMap<usize, StructOrTuple>> = HashMap::new(); // input_id -> parent position (0,1,etc) -> dependencies
    for (index, parent_id) in parent_ids.iter().enumerate() {
        if inputs.contains(parent_id) {
            // Parent is an input
            input_taint.entry(*next_stmt_id).or_default().insert(*parent_id);
            parent_input_dependencies.entry(*parent_id).or_default().insert(index, StructOrTuple::new_completely_dependent());
        }
        else if let Some(parent_taints) = input_taint.get(parent_id).cloned() {
            // Otherwise, extend the parent's
            input_taint.entry(*next_stmt_id).or_default().extend(parent_taints);
            if let Some(parent_dependencies) = input_dependencies.get(parent_id) {
                // If the parent has dependencies for the input it's taintd by, add them
                for (input_id, parent_dependencies_on_input) in parent_dependencies {
                    parent_input_dependencies.entry(*input_id).or_default().insert(index, parent_dependencies_on_input.clone());
                }
            }
        }
    }

    // Calculate input dependencies
    let input_taint_entry = input_taint.entry(*next_stmt_id).or_default();
    let input_dependencies_entry = input_dependencies.entry(*next_stmt_id).or_default();
    match node {
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
                // Parent is taintd by input but has no dependencies, delete
                input_dependencies_entry.remove(input_id);
            }
        }
        // Alters parent in a predicatable way
        HydroNode::Chain { .. } => {
            assert_eq!(parent_ids.len(), 2, "Node {:?} has the wrong number of parents.", node);
            // [a,b] chain [c,d] = [a,b,c,d]. Take the intersection of dependencies of the two parents
            for input_id in input_taint_entry.iter() {
                if let Some(parent_dependencies_on_input) = parent_input_dependencies.get(input_id) {
                    if let Some(parent1_dependency) = parent_dependencies_on_input.get(&0) {
                        if let Some(parent2_dependency) = parent_dependencies_on_input.get(&1) {
                            if let Some(intersection) = StructOrTuple::intersect(parent1_dependency, parent2_dependency) {
                                input_dependencies_entry.insert(*input_id, intersection);
                                continue;
                            }
                        }
                    }
                }
                // At least one parent has no dependencies or there's no overlap, delete
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
                input_dependencies_entry.remove(&input_id);
            }
        }
        HydroNode::Join { .. } => {
            assert_eq!(parent_ids.len(), 2, "Node {:?} has the wrong number of parents.", node);
            // [(a,b)] join [(a,c)] = [(a,(b,c)]
            for input_id in input_taint_entry.iter() {
                if let Some(parent_dependencies_on_input) = parent_input_dependencies.get(input_id) {
                    let mut new_dependency = StructOrTuple::default();
                    // Set a to shared dependencies between 0th index of parents
                    if let Some(parent1_dependency) = parent_dependencies_on_input.get(&0) {
                        if let Some(parent1_a_dependency) = parent1_dependency.get_dependency(&vec!["0".to_string()]) {
                            if let Some(parent2_dependency) = parent_dependencies_on_input.get(&1) {
                                if let Some(parent2_a_dependency) = parent2_dependency.get_dependency(&vec!["0".to_string()]) {
                                    if let Some(intersection) = StructOrTuple::intersect(&parent1_a_dependency, &parent2_a_dependency) {
                                        new_dependency.set_dependencies(&vec!["0".to_string()], &intersection, &vec![]);
                                    }
                                }
                            }
                        }
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
                input_dependencies_entry.remove(&input_id);
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
                // Parent is taintd by input but has no dependencies, delete
                input_dependencies_entry.remove(&input_id);
            }
        }
        // Based on f
        HydroNode::Map { f, .. }
        | HydroNode::FilterMap { f, .. }
         => {
            assert_eq!(parent_ids.len(), 1, "Node {:?} has the wrong number of parents.", node);
            // Analyze if we haven't yet
            let syn_analysis_results = syn_analysis.entry(*next_stmt_id).or_insert_with(|| {
                let mut analyzer = AnalyzeClosure::default();
                analyzer.visit_expr(&f.0);
                analyzer.output_dependencies.clone()
            });
            for input_id in input_taint_entry.iter() {
                if let Some(parent_dependencies_on_input) = parent_input_dependencies.get(input_id) {
                    if let Some(parent_dependency) = parent_dependencies_on_input.get(&0) {
                        // Project the parent's dependencies based on how f transforms the output
                        if let Some(projected_dependencies) = StructOrTuple::project_parent(parent_dependency, &syn_analysis_results) {
                            println!("Node {:?} input {:?} has projected dependencies: {:?}", next_stmt_id, input_id, projected_dependencies);
                            input_dependencies_entry.insert(*input_id, projected_dependencies);
                            continue;
                        }
                    }
                }
                // Parent is taintd by input but has no dependencies, delete
                input_dependencies_entry.remove(&input_id);
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
                // Parent is taintd by input but has no dependencies, delete
                input_dependencies_entry.remove(&input_id);
            }
        }
        // No dependencies on the parent (or no parent)
        HydroNode::Reduce { .. }
        | HydroNode::Fold { .. }
        | HydroNode::FlatMap { .. }
        | HydroNode::Source { .. } => {
            input_dependencies_entry.clear();
        }
        HydroNode::Placeholder { .. }
        | HydroNode::Counter { .. } => {
            panic!("Unexpected node type {:?} in input dependency analysis.", node);
        }
    }
}

fn input_dependency_analysis(
    ir: &mut [HydroLeaf],
    cluster_to_partition: LocationId,
    cycle_source_to_sink_input: &HashMap<usize, usize>,
) -> (HashMap<usize, HashSet<usize>>, HashMap<usize, HashMap<usize, StructOrTuple>>) {
    let mut metadata = InputDependencyMetadata {
        cluster_to_partition: cluster_to_partition.clone(),
        inputs: input_analysis(ir, cluster_to_partition),
        input_taint: HashMap::new(),
        input_dependencies: HashMap::new(),
        syn_analysis: HashMap::new(),
    };

    let mut num_iters = 0;
    let mut prev_hash = None;
    loop {
        println!("Input dependency analysis iteration {}", num_iters);

        traverse_dfir(
            ir,
            |_, _| {}, // Don't need to analyze leaves since they don't output anyway
            |node, next_stmt_id| {
                input_dependency_analysis_node(node, next_stmt_id, &mut metadata, cycle_source_to_sink_input);
            },
        );

        // Check if we've hit fixpoint
        let mut hasher = DefaultHasher::new();
        metadata.hash(&mut hasher);
        let hash = hasher.finish();
        if let Some(prev) = prev_hash {
            if prev == hash {
                break;
            }
        }
        prev_hash = Some(hash);
        num_iters += 1;
    }

    (metadata.input_taint, metadata.input_dependencies)
}


#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use hydro_lang::{deploy::DeployRuntime, ir::deep_clone, location::LocationId, rewrites::persist_pullup::persist_pullup, FlowBuilder, Location};
    use stageleft::{q, RuntimeData};

    use crate::{partition_node_analysis::input_dependency_analysis, partition_syn_analysis::StructOrTuple, repair::{cycle_source_to_sink_input, inject_id, inject_location}};

    fn test_input(builder: FlowBuilder<'_>, cluster_to_partition: LocationId, op_expected_dependencies: StructOrTuple) {
        let mut cycle_data = HashMap::new();
        let built = builder.optimize_with(persist_pullup)
            .optimize_with(inject_id)
            .optimize_with(|ir| {
                cycle_data = cycle_source_to_sink_input(ir);
                inject_location(ir, &cycle_data);
            })
            .into_deploy::<DeployRuntime>();
        let mut ir = deep_clone(built.ir());
        let (actual_taint, actual_dependencies) = input_dependency_analysis(&mut ir, cluster_to_partition, &cycle_data);

        // println!("Actual taint: {:?}", actual_taint);
        // println!("Actual dependencies: {:?}", actual_dependencies);

        let expected_taint = HashMap::from([
            (3, HashSet::from([])), // Network
            (4, HashSet::from([3])), // The implicit map following Network, imposed by broadcast_bincode_anonymous
            (5, HashSet::from([3])), // The operator being tested
        ]);
        
        let mut implicit_map_dependencies = StructOrTuple::default();
        implicit_map_dependencies.set_dependency(&vec![], vec!["1".to_string()]);

        let expected_dependencies = HashMap::from([
            (3, HashMap::new()),
            (4, HashMap::from([(3, implicit_map_dependencies)])),
            (5, HashMap::from([(3, op_expected_dependencies)])),
        ]);

        assert_eq!(actual_taint, expected_taint);
        assert_eq!(actual_dependencies, expected_dependencies);

        let _ = built.compile(&RuntimeData::new("FAKE"));
    }

    #[test]
    fn test_input_map() {
        let builder = FlowBuilder::new();
        let cluster1 = builder.cluster::<()>();
        let cluster2 = builder.cluster::<()>();
        cluster1
            .source_iter(q!([(1, 2)]))
            .broadcast_bincode_anonymous(&cluster2)
            .map(q!(|(a,b)| (b, a+2)))
            .for_each(q!(|(b, a2)| {
                println!("b: {}, a+2: {}", b, a2);
            }));
        
        let mut op_expected_dependency = StructOrTuple::default();
        op_expected_dependency.set_dependency(&vec!["0".to_string()], vec!["1".to_string(), "1".to_string()]);

        test_input(builder, cluster2.id(), op_expected_dependency);
    }
}