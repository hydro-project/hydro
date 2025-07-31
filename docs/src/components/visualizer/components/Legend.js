/**
 * Legend Component for Graph Visualizer
 * 
 * Displays a color-coded legend for different node types from JSON data
 */

import React from 'react';
import { generateNodeColors } from '../utils/utils.js';
import { DockablePanel, DOCK_POSITIONS } from './DockablePanel.js';
import { COMPONENT_COLORS } from '../utils/constants.js';

export function Legend({ colorPalette = 'Set3', graphData, onPositionChange }) {
  // Get legend data from the graph JSON, fallback to default if not provided
  const legendData = graphData?.legend || {
    title: "Node Types",
    items: [
      { type: "Source", label: "Source" },
      { type: "Transform", label: "Transform" },
      { type: "Sink", label: "Sink" },
      { type: "Network", label: "Network" },
      { type: "Aggregation", label: "Aggregation" },
      { type: "Join", label: "Join" },
      { type: "Tee", label: "Tee" }
    ]
  };

  return (
    <DockablePanel
      id="legend"
      title={legendData.title}
      defaultPosition={DOCK_POSITIONS.TOP_RIGHT}
      defaultDocked={true}
      defaultCollapsed={false}
      onPositionChange={onPositionChange}
      minWidth={200}
      minHeight={150}
    >
      <div>
        {legendData.items.map(item => {
          const colors = generateNodeColors(item.type, colorPalette);
          return (
            <div key={item.type} style={{
              display: 'flex',
              alignItems: 'center',
              margin: '3px 0',
              fontSize: '10px'
            }}>
              <div 
                style={{
                  width: '12px',
                  height: '12px',
                  borderRadius: '2px',
                  marginRight: '6px',
                  border: `1px solid ${COMPONENT_COLORS.BORDER_MEDIUM}`,
                  flexShrink: 0,
                  backgroundColor: colors.primary,
                  borderColor: colors.border
                }}
              />
              <span>{item.label}</span>
            </div>
          );
        })}
      </div>
    </DockablePanel>
  );
}
