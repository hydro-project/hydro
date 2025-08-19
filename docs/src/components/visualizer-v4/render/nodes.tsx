/**
 * @fileoverview Node components aggregator
 *
 * Split large monolithic node file into focused components.
 * Keeps the public API stable via re-exports and nodeTypes map.
 */

export { StandardNode, MemoStandardNode } from './StandardNode';
export { ContainerNode, MemoContainerNode } from './ContainerNode';

import { MemoStandardNode as StandardNodeComp } from './StandardNode';
import { MemoContainerNode as ContainerNodeComp } from './ContainerNode';

export const nodeTypes = {
  standard: StandardNodeComp,
  container: ContainerNodeComp,
};