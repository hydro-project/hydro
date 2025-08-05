/**
 * @fileoverview Components exports for the vis system
 */
export { FileDropZone } from './FileDropZone';
export { LayoutControls } from './LayoutControls';
export { InfoPanel } from './InfoPanel';
export { Legend } from './Legend';
export { HierarchyTree } from './HierarchyTree';
export { GroupingControls } from './GroupingControls';
export { CollapsibleSection } from './CollapsibleSection';
export { DockablePanel, PANEL_POSITIONS } from './DockablePanel';
export type { InfoPanelProps, LegendProps, HierarchyTreeProps, CollapsibleSectionProps, DockablePanelProps, HierarchyTreeNode, LegendData, LegendItem, GroupingOption, PanelPosition, BaseComponentProps } from './types';
export type { LayoutControlsProps } from './LayoutControls';
export declare function createDefaultLegendData(): {
    title: string;
    items: {
        type: string;
        label: string;
        description: string;
    }[];
};
//# sourceMappingURL=index.d.ts.map