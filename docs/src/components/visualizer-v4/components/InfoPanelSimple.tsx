/**
 * Simple InfoPanel Component - for debugging
 */

import React from 'react';

export interface SimpleInfoPanelProps {
  visualizationState: any;
  legendData?: any;
  hierarchyChoices?: any[];
  currentGrouping?: string | null;
  onGroupingChange?: (groupingId: string) => void;
  colorPalette?: string;
}

export function SimpleInfoPanel({
  visualizationState,
  legendData,
  hierarchyChoices = [],
  currentGrouping,
  onGroupingChange,
  colorPalette = 'Set3'
}: SimpleInfoPanelProps) {
  return (
    <div style={{ padding: '16px', border: '1px solid #ddd', borderRadius: '4px', backgroundColor: '#f9f9f9' }}>
      <h4>InfoPanel (Simple Version)</h4>
      <p>InfoPanel component loaded successfully</p>
      <p>Hierarchy choices: {Array.isArray(hierarchyChoices) ? hierarchyChoices.length : 'undefined'}</p>
      <p>Current grouping: {currentGrouping || 'none'}</p>
      <p>Color palette: {colorPalette}</p>
      <p>Legend data: {legendData ? 'provided' : 'not provided'}</p>
    </div>
  );
}
