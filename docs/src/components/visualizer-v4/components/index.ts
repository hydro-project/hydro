/**
 * @fileoverview Components exports for the vis system
 */

export { FileDropZone } from './FileDropZone';
export { LayoutControls } from './LayoutControls';

// InfoPanel system components
export { InfoPanel } from './InfoPanel';
export { Legend } from './Legend';
export { HierarchyTree } from './HierarchyTree';
export { GroupingControls } from './GroupingControls';
export { CollapsibleSection } from './CollapsibleSection';
export { DockablePanel } from './DockablePanel';
export { PANEL_POSITIONS } from './types';

// Visual Configuration Panel
export { VisualConfigPanel } from './VisualConfigPanel';

// Control components
export * from './controls';

// Types
export type {
  InfoPanelProps,
  LegendProps,
  HierarchyTreeProps,
  CollapsibleSectionProps,
  DockablePanelProps,
  HierarchyTreeNode,
  LegendData,
  LegendItem,
  GroupingOption,
  PanelPosition,
  BaseComponentProps,
  VisualConfigState,
  VisualConfigPanelProps
} from './types';

export type { LayoutControlsProps } from './LayoutControls';

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
