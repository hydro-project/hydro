/**
 * @fileoverview Handles renderer component
 *
 * Renders ReactFlow handles based on the current handle configuration.
 */

import React from 'react';
import { Handle, Position } from '@xyflow/react';
import { getHandleConfig, HANDLE_STYLES } from './handleConfig';

export function HandlesRenderer() {
  const config = getHandleConfig();

  if (config.enableContinuousHandles) {
    // ReactFlow v12 continuous handles - connections anywhere on perimeter
    return (
      <>
        <Handle
          type="source"
          position={Position.Top}
          style={HANDLE_STYLES.continuous}
          isConnectable={true}
        />
        <Handle
          type="target"
          position={Position.Top}
          style={HANDLE_STYLES.continuous}
          isConnectable={true}
        />
      </>
    );
  }

  // Discrete handles if configured
  return (
    <>
      {config.sourceHandles.map(handle => (
        <Handle
          key={handle.id}
          id={handle.id}
          type="source"
          position={handle.position}
          style={handle.style}
          isConnectable={true}
        />
      ))}
      {config.targetHandles.map(handle => (
        <Handle
          key={handle.id}
          id={handle.id}
          type="target"
          position={handle.position}
          style={handle.style}
          isConnectable={true}
        />
      ))}
    </>
  );
}
