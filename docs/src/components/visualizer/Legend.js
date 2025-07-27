/**
 * Legend Component
 * 
 * Displays node type and location legends
 */

import React from 'react';
import { generateNodeColors, generateLocationColor, generateLocationBorderColor, colorPalettes } from './colorUtils.js';
import styles from '../../pages/visualizer.module.css';

export function Legend({ colorPalette, locationData }) {
  return (
    <div className={styles.unifiedLegend}>
      <h4>Legend</h4>
      
      <div className={styles.legendSection}>
        <strong>Node Types:</strong>
        {['Source', 'Transform', 'Join', 'Aggregation', 'Network', 'Sink', 'Tee'].map(type => {
          const colors = generateNodeColors(type, colorPalette);
          return (
            <div key={type} className={styles.legendItem}>
              <div 
                className={styles.legendColor}
                style={{ background: colors.primary, borderColor: colors.border }}
              />
              <span>{type}</span>
            </div>
          );
        })}
      </div>

      {locationData.size > 0 && (
        <div className={styles.legendSection}>
          <strong>Locations:</strong>
          {Array.from(locationData.entries()).map(([locationId, location]) => {
            const bgColor = generateLocationColor(locationId, locationData.size, colorPalette);
            const borderColor = generateLocationBorderColor(locationId, locationData.size, colorPalette);
            return (
              <div key={locationId} className={styles.legendItem}>
                <div 
                  className={styles.locationLegendColor}
                  style={{ background: bgColor, borderColor: borderColor }}
                />
                <span>{location.label || location.name || `Location ${locationId}`}</span>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
