import React, { createContext, useContext } from 'react';

export type EdgeStyleKind = 'bezier' | 'straight' | 'smoothstep';

export interface StyleConfig {
  edgeStyle?: EdgeStyleKind;
  edgeColor?: string;
  edgeWidth?: number;
  edgeDashed?: boolean;

  nodeBorderRadius?: number;
  nodePadding?: number;
  nodeFontSize?: number; // px

  containerBorderRadius?: number;
  containerBorderWidth?: number;
  containerShadow?: 'LIGHT' | 'MEDIUM' | 'LARGE' | 'NONE';
}

const defaultStyleConfig: StyleConfig = {
  edgeStyle: 'bezier',
  edgeColor: '#1976d2',
  edgeWidth: 2,
  edgeDashed: false,

  nodeBorderRadius: 4,
  nodePadding: 12,
  nodeFontSize: 12,

  containerBorderRadius: 8,
  containerBorderWidth: 2,
  containerShadow: 'LIGHT'
};

const StyleConfigContext = createContext<StyleConfig>(defaultStyleConfig);

export function StyleConfigProvider({ value, children }: { value?: StyleConfig; children: React.ReactNode }) {
  return (
    <StyleConfigContext.Provider value={{ ...defaultStyleConfig, ...(value || {}) }}>
      {children}
    </StyleConfigContext.Provider>
  );
}

export function useStyleConfig(): StyleConfig {
  return useContext(StyleConfigContext);
}
