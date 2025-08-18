/**
 * @fileoverview Components exports for the vis system
 */
export { FileDropZone } from './FileDropZone.js';
// InfoPanel system components
export { InfoPanel } from './InfoPanel.js';
export { Legend } from './Legend.js';
export { HierarchyTree } from './HierarchyTree.js';
export { GroupingControls } from './GroupingControls.js';
export { CollapsibleSection } from './CollapsibleSection.js';
export { DockablePanel, PANEL_POSITIONS } from './DockablePanel.js';
// Utility functions for InfoPanel integration
export function createDefaultLegendData() {
    return {
        title: "Node Types",
        items: [
            { type: "Source", label: "Source", description: "Data input nodes" },
            { type: "Transform", label: "Transform", description: "Data transformation nodes" },
            { type: "Sink", label: "Sink", description: "Data output nodes" },
            { type: "Network", label: "Network", description: "Network communication nodes" },
            { type: "Aggregation", label: "Aggregation", description: "Data aggregation nodes" },
            { type: "Join", label: "Join", description: "Data joining nodes" },
            { type: "Tee", label: "Tee", description: "Data splitting nodes" }
        ]
    };
}
//# sourceMappingURL=index.js.map