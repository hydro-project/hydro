Building the Bridges

This is the approach I suggest. Everything in the core/ directory stays as is for now. We rewrite the logic on either side.

**JSON -> VisState**
- VisState should capture all information in the JSON file internally

**VisState -> ELK**
ELK should be given:
- visible nodes (both graphNodes and collapsed containers, which are just nodes) with no distinction between them
- visible container hierarchy (parent/child relationships)
- visible edges and hyperedges with no distinction between them.
- Nothing marked hidden

**ELK -> VisState**
VisState should capture the decisions made by ELK:
- dimensions of containers (to be cached first time, and validated subsequently as unchanged)
- locations of nodes and containers (to be passed along to ReactFlow)

**VisState -> ReactFlow**
ReactFlow should be given:
- locations and dimensions of containers, nodes
- edges
- labels, node and edge styles, and other display metadata captured from the JSON file

**ReactFlow -> VisState**
- node collapse
- node expand
- position change on drag

Am I missing anything?
