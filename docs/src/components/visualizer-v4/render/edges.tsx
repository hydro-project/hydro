/**
 * @fileoverview Edge Components Aggregator
 *
 * Thin module that re-exports edge components and preserves edgeTypes mapping.
 */

import { MemoFloatingEdge } from './FloatingEdge';
import { MemoStandardEdge } from './StandardEdge';
import { MemoHyperEdge } from './HyperEdge';

// Re-export individual edge components for compatibility
export { StandardEdge } from './StandardEdge';
export { HyperEdge } from './HyperEdge';

// Export map for ReactFlow edgeTypes (public API stability)
export const edgeTypes = {
  standard: MemoStandardEdge,
  hyper: MemoHyperEdge,
  floating: MemoFloatingEdge,
};
