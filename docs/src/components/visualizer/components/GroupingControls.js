/**
 * Grouping Controls Component
 * 
 * Dropdown control for selecting grouping hierarchy (Location, Backtrace, etc.)
 */

import React from 'react';
import styles from '../../../pages/visualizer.module.css';

export function GroupingControls({ 
  hierarchyChoices, 
  currentGrouping, 
  onGroupingChange 
}) {
  // If there's only one choice or no choices, show disabled dropdown
  const isDisabled = !hierarchyChoices || hierarchyChoices.length <= 1;
  
  return (
    <div className={styles.groupingControls}>
      <label className={styles.controlLabel}>Group by:</label>
      <select 
        className={`${styles.groupingSelect} ${isDisabled ? styles.disabled : ''}`}
        value={currentGrouping || ''} 
        onChange={(e) => onGroupingChange(e.target.value)}
        disabled={isDisabled}
      >
        {hierarchyChoices && hierarchyChoices.length > 0 ? (
          hierarchyChoices.map((choice) => (
            <option key={choice.id} value={choice.id}>
              {choice.name}
            </option>
          ))
        ) : (
          <option value="">No grouping available</option>
        )}
      </select>
    </div>
  );
}
