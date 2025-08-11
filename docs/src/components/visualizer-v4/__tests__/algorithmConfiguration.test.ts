import { describe, it, expect } from 'vitest';
import { VisualizationEngine } from '../core/VisualizationEngine';
import { VisualizationState } from '../core/VisState';
import { ELKBridge } from '../bridges/ELKBridge';

describe('Algorithm Configuration Consistency', () => {
  it('should default to mrtree algorithm in VisualizationEngine', () => {
    const visState = new VisualizationState();
    const engine = new VisualizationEngine(visState);
    
    // Check that the engine's internal config uses mrtree by default
    const config = (engine as any).config;
    expect(config.layoutConfig.algorithm).toBe('mrtree');
  });

  it('should default to mrtree algorithm in ELKBridge', () => {
    const bridge = new ELKBridge();
    
    // Check that the bridge's internal config uses mrtree by default
    const config = (bridge as any).layoutConfig;
    expect(config.algorithm).toBe('mrtree');
  });

  it('should respect explicit algorithm override', () => {
    const visState = new VisualizationState();
    const engine = new VisualizationEngine(visState, {
      layoutConfig: {
        algorithm: 'layered'
      }
    });
    
    // Check that explicit override works
    const config = (engine as any).config;
    expect(config.layoutConfig.algorithm).toBe('layered');
  });

  it('should allow ELKBridge algorithm override', () => {
    const bridge = new ELKBridge({ algorithm: 'force' });
    
    // Check that explicit override works for ELKBridge
    const config = (bridge as any).layoutConfig;
    expect(config.algorithm).toBe('force');
  });
});
