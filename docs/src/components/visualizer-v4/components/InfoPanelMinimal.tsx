/**
 * @fileoverview Minimal InfoPanel Component for debugging
 * 
 * This is a minimal version to isolate the hierarchyChoices error
 */

import React from 'react';

export interface InfoPanelMinimalProps {
  visualizationState?: any;
  legendData?: any;
  hierarchyChoices?: any[];
  currentGrouping?: string | null;
  onGroupingChange?: (groupingId: string) => void;
  colorPalette?: string;
}

export function InfoPanelMinimal({
  visualizationState,
  legendData = {},
  hierarchyChoices = [],
  currentGrouping,
  onGroupingChange,
  colorPalette = 'Set3'
}: InfoPanelMinimalProps) {
  
  // Ensure hierarchyChoices is always an array
  const safeHierarchyChoices = Array.isArray(hierarchyChoices) ? hierarchyChoices : [];

  // Get the current grouping name for the section title
  const currentGroupingName = safeHierarchyChoices.find(choice => choice?.id === currentGrouping)?.name || 'Container';

  return (
    <div style={{
      padding: '16px',
      border: '1px solid #ddd',
      borderRadius: '8px',
      backgroundColor: '#f9f9f9'
    }}>
      <h3 style={{ margin: '0 0 12px 0', fontSize: '14px' }}>InfoPanel Debug</h3>
      <div style={{ fontSize: '12px', color: '#666' }}>
        <p>Hierarchy choices: {safeHierarchyChoices.length}</p>
        <p>Current grouping: {currentGrouping || 'none'}</p>
        <p>Grouping name: {currentGroupingName}</p>
        <p>Color palette: {colorPalette}</p>
        <p>Has visualization state: {visualizationState ? 'yes' : 'no'}</p>
        <p>Legend keys: {Object.keys(legendData || {}).length}</p>
      </div>
      
      {safeHierarchyChoices.length > 0 && (
        <div style={{ marginTop: '12px' }}>
          <h4 style={{ margin: '0 0 8px 0', fontSize: '12px' }}>Available Groupings:</h4>
          <ul style={{ margin: 0, paddingLeft: '16px', fontSize: '10px' }}>
            {safeHierarchyChoices.map((choice, index) => (
              <li key={choice?.id || index}>
                {choice?.name || 'Unnamed'} ({choice?.id || 'no-id'})
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
