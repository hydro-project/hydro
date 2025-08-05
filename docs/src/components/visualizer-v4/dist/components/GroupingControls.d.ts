/**
 * @fileoverview GroupingControls Component
 *
 * Provides controls for selecting different hierarchical groupings.
 */
import React from 'react';
import { GroupingOption } from './types';
export interface GroupingControlsProps {
    hierarchyChoices?: GroupingOption[];
    currentGrouping?: string | null;
    onGroupingChange?: (groupingId: string) => void;
    compact?: boolean;
    disabled?: boolean;
    className?: string;
    style?: React.CSSProperties;
}
export declare function GroupingControls({ hierarchyChoices, currentGrouping, onGroupingChange, compact, disabled, className, style }: GroupingControlsProps): import("react/jsx-runtime").JSX.Element;
//# sourceMappingURL=GroupingControls.d.ts.map