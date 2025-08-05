/**
 * @fileoverview InfoPanel Component Types
 *
 * TypeScript interfaces for the InfoPanel system components.
 */
import { VisualizationState } from '../core/VisState';
export interface BaseComponentProps {
    className?: string;
    style?: React.CSSProperties;
}
export interface GroupingOption {
    id: string;
    name: string;
}
export interface LegendItem {
    type: string;
    label: string;
    description?: string;
}
export interface LegendData {
    title: string;
    items: LegendItem[];
}
export interface InfoPanelProps extends BaseComponentProps {
    visualizationState: VisualizationState;
    legendData?: LegendData;
    hierarchyChoices?: GroupingOption[];
    currentGrouping?: string | null;
    onGroupingChange?: (groupingId: string) => void;
    collapsedContainers?: Set<string>;
    onToggleContainer?: (containerId: string) => void;
    onPositionChange?: (panelId: string, position: PanelPosition) => void;
    colorPalette?: string;
    defaultCollapsed?: boolean;
}
export interface HierarchyTreeNode {
    id: string;
    label: string;
    children: HierarchyTreeNode[];
    nodeCount: number;
    isCollapsed?: boolean;
}
export interface HierarchyTreeProps extends BaseComponentProps {
    hierarchyTree: HierarchyTreeNode[];
    collapsedContainers?: Set<string>;
    onToggleContainer?: (containerId: string) => void;
    title?: string;
    showNodeCounts?: boolean;
    truncateLabels?: boolean;
    maxLabelLength?: number;
}
export interface LegendProps extends BaseComponentProps {
    legendData: LegendData;
    colorPalette?: string;
    nodeTypeConfig?: Record<string, any>;
    title?: string;
    compact?: boolean;
}
export declare const PANEL_POSITIONS: {
    readonly TOP_LEFT: "top-left";
    readonly TOP_RIGHT: "top-right";
    readonly BOTTOM_LEFT: "bottom-left";
    readonly BOTTOM_RIGHT: "bottom-right";
    readonly FLOATING: "floating";
};
export type PanelPosition = typeof PANEL_POSITIONS[keyof typeof PANEL_POSITIONS];
export interface DockablePanelProps extends BaseComponentProps {
    id: string;
    title: string;
    children: React.ReactNode;
    defaultPosition?: PanelPosition;
    defaultDocked?: boolean;
    defaultCollapsed?: boolean;
    onPositionChange?: (panelId: string, position: PanelPosition) => void;
    onDockChange?: (panelId: string, docked: boolean) => void;
    onCollapseChange?: (panelId: string, collapsed: boolean) => void;
    minWidth?: number;
    minHeight?: number;
    maxWidth?: number;
    maxHeight?: number;
}
export interface CollapsibleSectionProps extends BaseComponentProps {
    title: string;
    isCollapsed: boolean;
    onToggle: () => void;
    children: React.ReactNode;
    level?: number;
    showIcon?: boolean;
    disabled?: boolean;
}
export interface ContainerMetrics {
    totalContainers: number;
    expandedContainers: number;
    collapsedContainers: number;
    maxDepth: number;
    nodesByContainer: Map<string, Set<string>>;
}
export interface PanelState {
    position: PanelPosition;
    docked: boolean;
    collapsed: boolean;
    floatingPosition?: {
        x: number;
        y: number;
    };
    size?: {
        width: number;
        height: number;
    };
}
export interface InfoPanelState {
    panels: Record<string, PanelState>;
    legendCollapsed: boolean;
    hierarchyCollapsed: boolean;
    groupingCollapsed: boolean;
}
//# sourceMappingURL=types.d.ts.map