/**
 * Legend Component for Graph Visualizer
 * 
 * Displays a color-coded legend for different node types from JSON data
 */

import React, { useState } from 'react';
import { generateNodeColors } from '../utils/utils.js';
import styles from '../../../pages/visualizer.module.css';

export function Legend({ colorPalette = 'Set3', graphData }) {
  const [isCollapsed, setIsCollapsed] = useState(false);

  const toggleCollapsed = () => {
    setIsCollapsed(!isCollapsed);
  };

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
    <div className={styles.unifiedLegend}>
      <div className={styles.legendHeader}>
        <h4>{legendData.title}</h4>
        <button 
          className={styles.legendToggle}
          onClick={toggleCollapsed}
          title={isCollapsed ? "Expand legend" : "Collapse legend"}
        >
          {isCollapsed ? '⌄' : '⌃'}
        </button>
      </div>
      {!isCollapsed && (
        <div className={styles.legendSection}>
          {legendData.items.map(item => {
            const colors = generateNodeColors(item.type, colorPalette);
            return (
              <div key={item.type} className={styles.legendItem}>
                <div 
                  className={styles.legendColor}
                  style={{
                    backgroundColor: colors.primary,
                    borderColor: colors.border
                  }}
                />
                <span>{item.label}</span>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
