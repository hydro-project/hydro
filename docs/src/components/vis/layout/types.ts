/**
 * @fileoverview Layout types with proper TypeScript and centralized constants
 */

import type { VisualizationState, GraphNode, GraphEdge, Container, HyperEdge, Dimensions } from '../shared/types.js';
import type { ELKAlgorithm, ELKDirection } from '../shared/config.js';

// ============ Layout Configuration ============

export interface LayoutConfig {
  algorithm?: ELKAlgorithm;
  direction?: ELKDirection;
  spacing?: number;
  nodeSize?: { width: number; height: number };
}

// ============ Layout Results ============

export interface LayoutPosition {
  x: number;
  y: number;
}

export interface LayoutDimensions {
  width: number;
  height: number;
}

export interface PositionedNode extends GraphNode, LayoutPosition, LayoutDimensions {}
export interface PositionedEdge extends GraphEdge {
  points?: LayoutPosition[];
}
export interface PositionedContainer extends Container, LayoutPosition, LayoutDimensions {}
export interface PositionedHyperEdge extends HyperEdge {
  points?: LayoutPosition[];
}

export interface LayoutResult {
  nodes: PositionedNode[];
  edges: PositionedEdge[];
  containers: PositionedContainer[];
  hyperEdges: PositionedHyperEdge[];
}

// ============ Layout Engine Interface ============

export interface LayoutEngine {
  layout(
    nodes: GraphNode[],
    edges: GraphEdge[],
    containers: Container[],
    hyperEdges: HyperEdge[],
    config?: LayoutConfig
  ): Promise<LayoutResult>;
}

// ============ Layout Engine Options ============

export interface LayoutEngineOptions {
  enableCaching?: boolean;
  enableValidation?: boolean;
  logLevel?: 'none' | 'error' | 'warn' | 'info' | 'debug';
}

// ============ Layout Validation ============

export interface LayoutValidationResult {
  isValid: boolean;
  errors: LayoutValidationError[];
  warnings: LayoutValidationWarning[];
}

export interface LayoutValidationError {
  type: 'containment' | 'overlap' | 'bounds';
  message: string;
  nodeId?: string;
  containerId?: string;
  details?: Record<string, any>;
}

export interface LayoutValidationWarning {
  type: 'performance' | 'suboptimal' | 'compatibility';
  message: string;
  suggestion?: string;
  details?: Record<string, any>;
}

// ============ Layout Statistics ============

export interface LayoutStatistics {
  totalNodes: number;
  totalEdges: number;
  totalContainers: number;
  layoutDuration: number;
  validationResult?: LayoutValidationResult;
  cacheStats?: {
    hits: number;
    misses: number;
    size: number;
  };
}

// ============ Layout Events ============

export interface LayoutEventData {
  type: 'start' | 'progress' | 'complete' | 'error';
  progress?: number; // 0-100
  statistics?: LayoutStatistics;
  error?: Error;
}

export type LayoutEventCallback = (data: LayoutEventData) => void;

// ============ Advanced Layout Engine Interface ============

export interface AdvancedLayoutEngine extends LayoutEngine {
  // Configuration
  setOptions(options: LayoutEngineOptions): void;
  getOptions(): LayoutEngineOptions;
  
  // Caching
  clearCache(): void;
  getCacheStatistics(): { size: number; hits?: number; misses?: number };
  
  // Validation
  validateLayout(result: LayoutResult): LayoutValidationResult;
  
  // Events
  on(event: 'layout', callback: LayoutEventCallback): void;
  off(event: 'layout', callback: LayoutEventCallback): void;
  
  // Statistics
  getLastLayoutStatistics(): LayoutStatistics | null;
}
