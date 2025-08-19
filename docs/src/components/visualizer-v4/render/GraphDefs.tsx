/**
 * Lightweight presentational component for global SVG defs used by edges.
 * Extracted from FlowGraph to reduce render complexity without behavior changes.
 */
import React from 'react';

export const GraphDefs: React.FC = React.memo(() => (
  <svg width="0" height="0" style={{ position: 'absolute' }}>
    <defs>
      {/* Custom arrowhead markers for collection types */}
      <marker
        id="circle-filled"
        markerWidth="8"
        markerHeight="8"
        refX="6"
        refY="4"
        orient="auto"
        markerUnits="strokeWidth"
      >
        <circle cx="4" cy="4" r="3" fill="currentColor" />
      </marker>

      <marker
        id="diamond-open"
        markerWidth="10"
        markerHeight="8"
        refX="8"
        refY="4"
        orient="auto"
        markerUnits="strokeWidth"
      >
        <path d="M2,4 L5,1 L8,4 L5,7 Z" fill="none" stroke="currentColor" strokeWidth="1" />
      </marker>
    </defs>
  </svg>
));

GraphDefs.displayName = 'GraphDefs';
