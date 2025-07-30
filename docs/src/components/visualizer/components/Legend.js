/**
 * Legend Component for Graph Visualizer
 * 
 * Displays a color-coded legend for different node types
 */

import React from 'react';
import { generateNodeColors } from '../utils/utils.js';
import styles from '../../../pages/visualizer.module.css';

const nodeTypes = [
  'Source',
  'Transform', 
  'Sink',
  'Network',
  'Operator',
  'Join',
  'Union',
  'Filter'
];

export function Legend({ colorPalette = 'Set3' }) {
  return (
    <div className={styles.unifiedLegend}>
      <h4>Node Types</h4>
      <div className={styles.legendSection}>
        {nodeTypes.map(nodeType => {
          const colors = generateNodeColors(nodeType, colorPalette);
          return (
            <div key={nodeType} className={styles.legendItem}>
              <div 
                className={styles.legendColor}
                style={{
                  backgroundColor: colors.primary,
                  borderColor: colors.border
                }}
              />
              <span>{nodeType}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
