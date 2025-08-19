/**
 * @fileoverview EdgeStyleLegend Component
 * 
 * Displays a legend showing what different edge visual styles represent
 * in terms of stream properties (Bounded/Unbounded, TotalOrder/NoOrder, etc.)
 */

import React, { useMemo } from 'react';
import { COMPONENT_COLORS, TYPOGRAPHY } from '../shared/config';

interface EdgeStyleLegendProps {
  edgeStyleConfig?: {
    semanticMappings?: Record<string, Record<string, Record<string, string | number>>>;
    // Legacy support
    booleanPropertyPairs?: Array<{
      pair: [string, string];
      defaultStyle: string;
      altStyle: string;
      description: string;
    }>;
    singlePropertyMappings?: Record<string, string>;
    combinationRules?: any;
    propertyMappings?: Record<string, string>;
  };
  compact?: boolean;
  className?: string;
  style?: React.CSSProperties;
}

// Visual representations of each edge style
const EDGE_STYLE_SAMPLES = {
  // New numbered edge style system
  'edge_style_1': (
    <svg width="40" height="3" viewBox="0 0 40 3">
      <line x1="0" y1="1.5" x2="40" y2="1.5" stroke="#4a5568" strokeWidth="2"/>
    </svg>
  ),
  'edge_style_1_alt': (
    <svg width="40" height="3" viewBox="0 0 40 3">
      <line x1="0" y1="1.5" x2="40" y2="1.5" stroke="#4a5568" strokeWidth="2" strokeDasharray="4,4"/>
    </svg>
  ),
  'edge_style_2': (
    <svg width="40" height="3" viewBox="0 0 40 3">
      <line x1="0" y1="1.5" x2="40" y2="1.5" stroke="#4a5568" strokeWidth="1"/>
    </svg>
  ),
  'edge_style_2_alt': (
    <svg width="40" height="3" viewBox="0 0 40 3">
      <line x1="0" y1="1.5" x2="40" y2="1.5" stroke="#4a5568" strokeWidth="3"/>
    </svg>
  ),
  'edge_style_3': (
    <svg width="40" height="3" viewBox="0 0 40 3">
      <line x1="0" y1="1.5" x2="40" y2="1.5" stroke="#4a5568" strokeWidth="2"/>
    </svg>
  ),
  'edge_style_3_alt': (
    <svg width="40" height="3" viewBox="0 0 40 3">
      <line x1="0" y1="1.5" x2="40" y2="1.5" stroke="#4a5568" strokeWidth="2"/>
      <animateTransform attributeName="transform" type="translate" values="0,0;5,0;0,0" dur="1s" repeatCount="indefinite"/>
    </svg>
  ),
  'edge_style_4': (
    <svg width="40" height="5" viewBox="0 0 40 5">
      <line x1="0" y1="1" x2="40" y2="1" stroke="#4a5568" strokeWidth="1" strokeDasharray="8,2,2,2"/>
    </svg>
  ),
  'edge_style_5': (
    <svg width="40" height="3" viewBox="0 0 40 3">
      <line x1="0" y1="1.5" x2="40" y2="1.5" stroke="#4a5568" strokeWidth="2" strokeDasharray="2,2"/>
    </svg>
  ),
  
  // Legacy styles for backward compatibility
  'thick-stroke': (
    <svg width="40" height="3" viewBox="0 0 40 3">
      <line x1="0" y1="1.5" x2="40" y2="1.5" stroke="#4a5568" strokeWidth="3"/>
    </svg>
  ),
  'thin-stroke': (
    <svg width="40" height="3" viewBox="0 0 40 3">
      <line x1="0" y1="1.5" x2="40" y2="1.5" stroke="#4a5568" strokeWidth="1"/>
    </svg>
  ),
  'smooth-line': (
    <svg width="40" height="3" viewBox="0 0 40 3">
      <line x1="0" y1="1.5" x2="40" y2="1.5" stroke="#4a5568" strokeWidth="2"/>
    </svg>
  ),
  'wavy-line': (
    <svg width="40" height="10" viewBox="0 0 40 10">
      <path d="M0,5 Q10,0 20,5 T40,5" stroke="#4a5568" strokeWidth="2" fill="none"/>
    </svg>
  ),
  'dashed-animated': (
    <svg width="40" height="3" viewBox="0 0 40 3">
      <line x1="0" y1="1.5" x2="40" y2="1.5" stroke="#4a5568" strokeWidth="2" strokeDasharray="5,3"/>
    </svg>
  ),
  'double-line': (
    <svg width="40" height="5" viewBox="0 0 40 5">
      <line x1="0" y1="1" x2="40" y2="1" stroke="#4a5568" strokeWidth="1"/>
      <line x1="0" y1="4" x2="40" y2="4" stroke="#4a5568" strokeWidth="1"/>
    </svg>
  )
};

// Note: Removed unused PROPERTY_DESCRIPTIONS dead constant

function EdgeStyleLegendInner({
  edgeStyleConfig,
  compact = false,
  className = '',
  style
}: EdgeStyleLegendProps) {
  // Safety check for edgeStyleConfig
  if (!edgeStyleConfig) {
    return (
      <div className={`edge-style-legend-empty ${className}`} style={style}>
        <span style={{ 
          color: COMPONENT_COLORS.TEXT_DISABLED,
          fontSize: compact ? TYPOGRAPHY.UI_SMALL : TYPOGRAPHY.UI_MEDIUM,
          fontStyle: 'italic'
        }}>
          No edge style data available
        </span>
      </div>
    );
  }

  const legendStyle: React.CSSProperties = useMemo(() => ({
    fontSize: compact ? '9px' : '10px',
    ...style
  }), [compact, style]);

  const styles = useMemo(() => {
    const pairBoxStyle: React.CSSProperties = {
      display: 'flex',
      alignItems: 'flex-start',
      border: '1px solid #ddd',
      borderRadius: '4px',
      padding: '6px',
      margin: '3px 0',
      backgroundColor: '#fafafa'
    };

    const numberStyle: React.CSSProperties = {
      fontSize: '12px',
      fontWeight: 'bold',
      marginRight: '8px',
      minWidth: '16px',
      color: COMPONENT_COLORS.TEXT_PRIMARY,
      flexShrink: 0
    };

    const contentStyle: React.CSSProperties = { flex: 1 };

    const edgeRowStyle: React.CSSProperties = {
      display: 'flex',
      alignItems: 'center',
      margin: '1px 0',
      fontSize: compact ? TYPOGRAPHY.UI_SMALL : TYPOGRAPHY.UI_MEDIUM,
    };

    const sampleStyle: React.CSSProperties = {
      marginRight: '8px',
      minWidth: '48px',
      display: 'flex',
      alignItems: 'center'
    };

    const labelStyle: React.CSSProperties = {
      fontSize: '10px',
      color: COMPONENT_COLORS.TEXT_PRIMARY
    };

    return { pairBoxStyle, numberStyle, contentStyle, edgeRowStyle, sampleStyle, labelStyle };
  }, [compact]);

  // Helper function to render semantic mapping boxes
  const renderSemanticMappingBoxes = () => {
    if (!edgeStyleConfig.semanticMappings) return null;

    let groupNumber = 1;
    const allBoxes: JSX.Element[] = [];

    // Process each group
    Object.entries(edgeStyleConfig.semanticMappings).forEach(([groupName, group]) => {
      // Create group header
    const groupHeader = (
        <div key={`${groupName}-header`} style={{
      ...styles.edgeRowStyle,
          fontWeight: 'bold',
          marginTop: groupNumber > 1 ? '8px' : '0px',
          marginBottom: '2px',
          color: COMPONENT_COLORS.TEXT_PRIMARY
        }}>
          {groupNumber}. {groupName}
        </div>
      );
      allBoxes.push(groupHeader);

      // Process each option within the group
      Object.entries(group).forEach(([optionName, styleSettings], optionIndex) => {
        // Generate a visual sample based on the style settings
        const sample = generateVisualSample(styleSettings);
        
        const box = (
          <div key={`${groupName}-${optionName}`} style={{
            ...styles.pairBoxStyle,
            marginLeft: '16px', // Indent options under group
            marginBottom: '1px'
          }}>
            <div style={{
              ...styles.numberStyle,
              backgroundColor: 'transparent',
              color: COMPONENT_COLORS.TEXT_SECONDARY,
              fontSize: '10px',
              width: '12px'
            }}>
              {String.fromCharCode(97 + optionIndex)} {/* 'a', 'b', etc. */}
            </div>
            
            <div style={styles.contentStyle}>
              <div style={styles.edgeRowStyle}>
                <div style={styles.sampleStyle}>
                  {sample}
                </div>
                <span style={styles.labelStyle}>
                  {optionName}
                </span>
              </div>
            </div>
          </div>
        );
        
        allBoxes.push(box);
      });

      groupNumber++;
    });

    return allBoxes;
  };

  // Helper function to generate visual sample from style settings
  const generateVisualSample = (styleSettings: Record<string, string | number>) => {
  const linePattern = styleSettings['line-pattern'] as string || 'solid';
  // Default to thin to match graph default stroke width
  const lineWidth = (styleSettings['line-width'] as number) ?? 1;
    const animation = styleSettings['animation'] as string;
    const lineStyle = styleSettings['line-style'] as string || 'single';
    const halo = styleSettings['halo'] as string;
    const arrowhead = styleSettings['arrowhead'] as string;
    const waviness = styleSettings['waviness'] as string;

  let strokeDasharray = undefined;
    switch (linePattern) {
      case 'dashed':
        strokeDasharray = '8,4';
        break;
      case 'dotted':
    strokeDasharray = '2,2';
        break;
      case 'dash-dot':
        strokeDasharray = '8,2,2,2';
        break;
    }

    const haloColors = {
      'light-blue': '#e6f3ff',
      // Slightly stronger so it peeks beyond the stroke clearly
      'light-red': '#ffb3b3',
      'light-green': '#e6ffe6'
    };

    const haloColor = halo && halo !== 'none' ? haloColors[halo as keyof typeof haloColors] : undefined;

    // Helper to render a head marker according to arrowhead
    const renderHeadMarker = (x: number, y: number, color: string) => {
      switch (arrowhead) {
        case 'triangle-open':
          return <polygon points={`${x},${y} ${x-6},${y-4} ${x-6},${y+4}`} fill="none" stroke={color} strokeWidth={1}/>;
        case 'triangle-filled':
          return <polygon points={`${x},${y} ${x-6},${y-4} ${x-6},${y+4}`} fill={color} />;
        case 'circle-filled':
          return <circle cx={x-4} cy={y} r={3} fill={color}/>;
        case 'diamond-open':
          return <polygon points={`${x-2},${y} ${x-6},${y-3} ${x-10},${y} ${x-6},${y+3}`} fill="none" stroke={color} strokeWidth={1}/>;
        default:
          return null;
      }
    };

    // Helper to build a gentle wavy path centered on y
    const wavePathD = (y: number) => `M0,${y} Q10,${y - 3} 20,${y} T40,${y}`;

    const isWavy = waviness === 'wavy';

    if (lineStyle === 'double') {
      // Render double lines with extra height to avoid stroke clipping
      const height = Math.max(10, lineWidth + 6);
  // Keep a minimum separation of 2px between double lines
  const mid = Math.round(height / 2);
  const sep = Math.max(2, Math.round(lineWidth / 2));
  const y1 = mid - sep;
  const y2 = mid + sep;
      return (
        <svg width="40" height={height} viewBox={`0 0 40 ${height}`} style={{ overflow: 'visible' }}>
          {/* approximate head marker in legend */}
          {renderHeadMarker(34, y1, '#4a5568')}
          {haloColor && (<>
              {isWavy ? (
                <>
                  <path d={wavePathD(y1)} stroke={haloColor} strokeWidth={lineWidth + 4} fill="none" strokeDasharray={strokeDasharray} strokeLinecap="round" />
                  <path d={wavePathD(y2)} stroke={haloColor} strokeWidth={lineWidth + 4} fill="none" strokeDasharray={strokeDasharray} strokeLinecap="round" />
                </>
              ) : (
                <>
                  <line x1="0" y1={y1} x2="40" y2={y1} stroke={haloColor} strokeWidth={lineWidth + 4} strokeDasharray={strokeDasharray} strokeLinecap="round"/>
                  <line x1="0" y1={y2} x2="40" y2={y2} stroke={haloColor} strokeWidth={lineWidth + 4} strokeDasharray={strokeDasharray} strokeLinecap="round"/>
                </>
              )}
          </>)}
          {isWavy ? (
            <>
              <path d={wavePathD(y1)} stroke="#4a5568" strokeWidth={lineWidth} fill="none" strokeDasharray={strokeDasharray} strokeLinecap="round" />
              <path d={wavePathD(y2)} stroke="#4a5568" strokeWidth={lineWidth} fill="none" strokeDasharray={strokeDasharray} strokeLinecap="round" />
            </>
          ) : (
            <>
              <line x1="0" y1={y1} x2="40" y2={y1} stroke="#4a5568" strokeWidth={lineWidth} strokeDasharray={strokeDasharray} strokeLinecap="round">
                {animation === 'animated' && (
                  <animateTransform attributeName="transform" type="translate" values="0,0;5,0;0,0" dur="1s" repeatCount="indefinite"/>
                )}
              </line>
              <line x1="0" y1={y2} x2="40" y2={y2} stroke="#4a5568" strokeWidth={lineWidth} strokeDasharray={strokeDasharray} strokeLinecap="round">
                {animation === 'animated' && (
                  <animateTransform attributeName="transform" type="translate" values="0,0;5,0;0,0" dur="1s" repeatCount="indefinite"/>
                )}
              </line>
            </>
          )}
        </svg>
      );
    } else {
      // Render single line with extra height to avoid stroke clipping
      const height = Math.max(8, lineWidth + 4);
      const y = Math.round(height / 2);
      return (
        <svg width="40" height={height} viewBox={`0 0 40 ${height}`} style={{ overflow: 'visible' }}>
          {/* approximate head marker in legend */}
          {renderHeadMarker(34, y, '#4a5568')}
          {haloColor && (
            isWavy ? (
              <path d={wavePathD(y)} stroke={haloColor} strokeWidth={lineWidth + 4} fill="none" strokeDasharray={strokeDasharray} strokeLinecap="round" />
            ) : (
              <line x1="0" y1={y} x2="40" y2={y} stroke={haloColor} strokeWidth={lineWidth + 4} strokeDasharray={strokeDasharray} strokeLinecap="round"/>
            )
          )}
          {isWavy ? (
            <path d={wavePathD(y)} stroke="#4a5568" strokeWidth={lineWidth} fill="none" strokeDasharray={strokeDasharray} strokeLinecap="round" />
          ) : (
            <line x1="0" y1={y} x2="40" y2={y} stroke="#4a5568" strokeWidth={lineWidth} strokeDasharray={strokeDasharray} strokeLinecap="round">
              {animation === 'animated' && (
                <animateTransform attributeName="transform" type="translate" values="0,0;5,0;0,0" dur="1s" repeatCount="indefinite"/>
              )}
            </line>
          )}
        </svg>
      );
    }
  };

  // Helper function to render boolean pair boxes
  const renderBooleanPairBoxes = () => {
    if (!edgeStyleConfig.booleanPropertyPairs) return null;

    return edgeStyleConfig.booleanPropertyPairs.map((pairConfig, index) => {
      const [defaultProp, altProp] = pairConfig.pair;
      const defaultSample = EDGE_STYLE_SAMPLES[pairConfig.defaultStyle as keyof typeof EDGE_STYLE_SAMPLES];
      const altSample = EDGE_STYLE_SAMPLES[pairConfig.altStyle as keyof typeof EDGE_STYLE_SAMPLES];

      return (
        <div key={`pair-${index}`} style={styles.pairBoxStyle}>
          <div style={styles.numberStyle}>
            {index + 1}
          </div>
          
          <div style={styles.contentStyle}>
            <div style={styles.edgeRowStyle}>
              <div style={styles.sampleStyle}>
                {defaultSample || <span>—</span>}
              </div>
              <span style={styles.labelStyle}>
                {defaultProp}
              </span>
            </div>
            
            <div style={styles.edgeRowStyle}>
              <div style={styles.sampleStyle}>
                {altSample || <span>- -</span>}
              </div>
              <span style={styles.labelStyle}>
                {altProp}
              </span>
            </div>
          </div>
        </div>
      );
    });
  };

  // Helper function to render single property boxes
  const renderSinglePropertyBoxes = () => {
    if (!edgeStyleConfig.singlePropertyMappings) return null;

    return Object.entries(edgeStyleConfig.singlePropertyMappings).map(([property, styleTag], index) => {
      const sample = EDGE_STYLE_SAMPLES[styleTag as keyof typeof EDGE_STYLE_SAMPLES];
      const styleNumber = styleTag.replace('edge_style_', '');
      
      return (
        <div key={property} style={styles.pairBoxStyle}>
          <div style={styles.numberStyle}>
            {styleNumber}
          </div>
          
          <div style={styles.contentStyle}>
            <div style={styles.edgeRowStyle}>
              <div style={styles.sampleStyle}>
                {sample || <span>■</span>}
              </div>
              <span style={styles.labelStyle}>
                {property}
              </span>
            </div>
          </div>
        </div>
      );
    });
  };

  // Legacy support - render old format in simple boxes
  const renderLegacyBoxes = () => {
    if (!edgeStyleConfig.propertyMappings) return null;

    return Object.entries(edgeStyleConfig.propertyMappings).map(([property, styleTag], index) => {
      const sample = EDGE_STYLE_SAMPLES[styleTag as keyof typeof EDGE_STYLE_SAMPLES];
      
      return (
        <div key={property} style={styles.pairBoxStyle}>
          <div style={styles.numberStyle}>
            {index + 1}
          </div>
          
          <div style={styles.contentStyle}>
            <div style={styles.edgeRowStyle}>
              <div style={styles.sampleStyle}>
                {sample || <span>■</span>}
              </div>
              <span style={styles.labelStyle}>
                {property}
              </span>
            </div>
          </div>
        </div>
      );
    });
  };

  // Memoize sections to avoid re-computation on unrelated re-renders
  const semanticBoxes = useMemo(() => renderSemanticMappingBoxes(), [edgeStyleConfig?.semanticMappings, styles]);
  const booleanPairs = useMemo(() => renderBooleanPairBoxes(), [edgeStyleConfig?.booleanPropertyPairs, styles]);
  const singleProps = useMemo(() => renderSinglePropertyBoxes(), [edgeStyleConfig?.singlePropertyMappings, styles]);
  const legacyBoxes = useMemo(() => renderLegacyBoxes(), [edgeStyleConfig?.propertyMappings, styles]);

  return (
    <div className={`edge-style-legend ${className}`} style={legendStyle}>
      <div style={{
        fontWeight: 'bold',
        marginBottom: compact ? '6px' : '8px',
        fontSize: compact ? TYPOGRAPHY.UI_SMALL : TYPOGRAPHY.UI_MEDIUM,
        color: COMPONENT_COLORS.TEXT_PRIMARY
      }}>
        Edge Styles
      </div>
      
      {/* New semantic mappings system */}
      {semanticBoxes}
      
      {/* Legacy boolean pair system */}
      {booleanPairs}
      
      {/* Single properties */}
      {singleProps}
      
      {/* Legacy support */}
      {legacyBoxes}
    </div>
  );
}

export const EdgeStyleLegend = React.memo(EdgeStyleLegendInner);
