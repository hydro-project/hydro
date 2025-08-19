/**
 * @fileoverview Edge Components Aggregator
 *
 * Thin module that re-exports edge components and preserves edgeTypes mapping.
 */

import FloatingEdge, { MemoFloatingEdge } from './FloatingEdge';
import { StandardEdge, MemoStandardEdge } from './StandardEdge';
import { HyperEdge, MemoHyperEdge } from './HyperEdge';

// Re-export individual edge components for compatibility
export { StandardEdge } from './StandardEdge';
export { HyperEdge } from './HyperEdge';

// Export map for ReactFlow edgeTypes (public API stability)
export const edgeTypes = {
  standard: MemoStandardEdge,
  hyper: MemoHyperEdge,
  floating: MemoFloatingEdge,
};
