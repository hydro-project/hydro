/**
 * Layout Controls Component
 * 
 * Dropdown controls for layout algorithm and color palette selection
 */

import React from 'react';
import { elkLayouts } from './layoutConfigs.js';
import { colorPalettes } from './colorUtils.js';
import styles from '../../pages/visualizer.module.css';

export function LayoutControls({ currentLayout, onLayoutChange, colorPalette, onPaletteChange }) {
  return (
    <div className={styles.layoutControls}>
      <select 
        className={styles.layoutSelect}
        value={currentLayout} 
        onChange={(e) => onLayoutChange(e.target.value)}
      >
        {Object.keys(elkLayouts).map(key => (
          <option key={key} value={key}>{key.charAt(0).toUpperCase() + key.slice(1)}</option>
        ))}
      </select>
      
      <select 
        className={styles.paletteSelect}
        value={colorPalette} 
        onChange={(e) => onPaletteChange(e.target.value)}
      >
        {Object.keys(colorPalettes).map(key => (
          <option key={key} value={key}>{key}</option>
        ))}
      </select>
    </div>
  );
}
