/**
 * @fileoverview InfoPanel Component
 *
 * Combined info panel that displays grouping controls, legend, and container hierarchy
 * with collapsible sections for organizing the interface.
 */
import { InfoPanelProps } from './types';
export declare function InfoPanel({ visualizationState, legendData, hierarchyChoices, currentGrouping, onGroupingChange, collapsedContainers, onToggleContainer, onPositionChange, colorPalette, defaultCollapsed, className, style }: InfoPanelProps): import("react/jsx-runtime").JSX.Element;
export { Legend } from './Legend';
export { HierarchyTree } from './HierarchyTree';
export { GroupingControls } from './GroupingControls';
export { CollapsibleSection } from './CollapsibleSection';
export { DockablePanel, PANEL_POSITIONS } from './DockablePanel';
//# sourceMappingURL=InfoPanel.d.ts.map