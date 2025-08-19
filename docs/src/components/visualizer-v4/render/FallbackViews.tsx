/**
 * Presentational fallback views and overlays extracted from FlowGraph.
 * No behavior changes.
 */
import React from 'react';

export type CommonViewProps = {
  className?: string;
  containerStyle?: React.CSSProperties;
};

export const LoadingView: React.FC<CommonViewProps> = ({ className, containerStyle }) => (
  <div 
    className={className}
    style={{
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      background: '#f5f5f5',
      border: '1px solid #ddd',
      borderRadius: '8px',
      ...containerStyle
    }}
  >
    <div style={{ textAlign: 'center', color: '#666' }}>
      <div style={{ 
        width: '40px',
        height: '40px',
        margin: '0 auto 16px',
        border: '4px solid #f3f3f3',
        borderTop: '4px solid #3498db',
        borderRadius: '50%',
        animation: 'modernSpin 1s linear infinite'
      }} />
      <div style={{ fontSize: '18px', marginBottom: '8px' }}>
        Processing Graph Layout...
      </div>
      <div style={{ fontSize: '14px', color: '#999' }}>
        Large graphs may take a moment to compute
      </div>
    </div>
    <style>{`
      @keyframes modernSpin {
        0% { transform: rotate(0deg); }
        100% { transform: rotate(360deg); }
      }
    `}</style>
  </div>
);

export const ErrorView: React.FC<CommonViewProps & { message?: string }> = ({ className, containerStyle, message }) => (
  <div 
    className={className}
    style={{
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      background: '#ffe6e6',
      border: '1px solid #ff9999',
      borderRadius: '8px',
      ...containerStyle
    }}
  >
    <div style={{ textAlign: 'center', color: '#cc0000' }}>
      <div style={{ fontSize: '24px', marginBottom: '8px' }}>❌</div>
      <div><strong>Visualization Error:</strong></div>
      <div style={{ fontSize: '14px', marginTop: '4px' }}>{message}</div>
    </div>
  </div>
);

export const EmptyView: React.FC<CommonViewProps> = ({ className, containerStyle }) => (
  <div 
    className={className}
    style={{
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      background: '#f9f9f9',
      border: '1px solid #ddd',
      borderRadius: '8px',
      ...containerStyle
    }}
  >
    <div style={{ textAlign: 'center', color: '#666' }}>
      <div style={{ fontSize: '24px', marginBottom: '8px' }}>📊</div>
      <div>No visualization data</div>
    </div>
  </div>
);

export const UpdatingOverlay: React.FC = React.memo(() => (
  <div style={{
    position: 'absolute',
    top: '10px',
    right: '10px',
    background: 'rgba(255, 255, 255, 0.9)',
    padding: '8px 12px',
    borderRadius: '4px',
    border: '1px solid #ddd',
    fontSize: '12px',
    color: '#666'
  }}>
    🔄 Updating...
  </div>
));

UpdatingOverlay.displayName = 'UpdatingOverlay';
