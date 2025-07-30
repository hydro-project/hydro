/**
 * Layout Controls Component
 * 
 * Dropdown controls for layout algorithm, color palette, and grouping hierarchy selection
 */

import React from 'react';
import { GroupingControls } from './GroupingControls.js';
import styles from '../../../pages/visualizer.module.css';

const layoutOptions = {
  mrtree: 'MR Tree',
  layered: 'Layered',
  force: 'Force',
  stress: 'Stress',
  radial: 'Radial'
};

const paletteOptions = {
  Set3: 'Set3',
  Pastel1: 'Pastel1', 
  Dark2: 'Dark2'
};

export function LayoutControls({ 
  currentLayout, 
  onLayoutChange, 
  colorPalette, 
  onPaletteChange,
  hasCollapsedContainers,
  onCollapseAll,
  onExpandAll,
  hierarchyChoices,
  currentGrouping,
  onGroupingChange
}) {
  return (
    <div className={styles.layoutControls}>
      <select 
        className={styles.layoutSelect}
        value={currentLayout} 
        onChange={(e) => onLayoutChange(e.target.value)}
      >
        {Object.entries(layoutOptions).map(([key, label]) => (
          <option key={key} value={key}>{label}</option>
        ))}
      </select>

      <select 
        className={styles.paletteSelect}
        value={colorPalette} 
        onChange={(e) => onPaletteChange(e.target.value)}
      >
        {Object.entries(paletteOptions).map(([key, label]) => (
          <option key={key} value={key}>{label}</option>
        ))}
      </select>
      
      <GroupingControls
        hierarchyChoices={hierarchyChoices}
        currentGrouping={currentGrouping}
        onGroupingChange={onGroupingChange}
      />
      
      <button 
        className={styles.containerButton}
        onClick={hasCollapsedContainers ? onExpandAll : onCollapseAll}
        title={hasCollapsedContainers ? 'Expand All Containers' : 'Collapse All Containers'}
      >
        {hasCollapsedContainers ? '⊞' : '⊟'}
      </button>
    </div>
  );
}
