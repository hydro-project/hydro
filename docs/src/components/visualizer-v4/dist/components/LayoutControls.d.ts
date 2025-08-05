/**
 * Layout Controls Component for Vis System
 *
 * Ported from the visualizer system and adapted for the new VisState architecture.
 * Provides controls for layout, color palette, container operations, and viewport management.
 */
import React from 'react';
import type { VisualizationState } from '../core/VisState';
export interface LayoutControlsProps {
    visualizationState: VisualizationState;
    currentLayout?: string;
    onLayoutChange?: (layout: string) => void;
    colorPalette?: string;
    onPaletteChange?: (palette: string) => void;
    onCollapseAll?: () => void;
    onExpandAll?: () => void;
    autoFit?: boolean;
    onAutoFitToggle?: (enabled: boolean) => void;
    onFitView?: () => void;
    className?: string;
    style?: React.CSSProperties;
}
export declare function LayoutControls({ visualizationState, currentLayout, onLayoutChange, colorPalette, onPaletteChange, onCollapseAll, onExpandAll, autoFit, onAutoFitToggle, onFitView, className, style }: LayoutControlsProps): JSX.Element;
//# sourceMappingURL=LayoutControls.d.ts.map