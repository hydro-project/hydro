// Local shim for @hydro-project/hydroscope used in CI/docs build when the real
// package is not available in the environment. Provides a minimal Hydroscope
// component and helpers so the docs build can succeed.

import React from "react";

export type HydroscopeProps = {
  data?: any;
  height?: string | number;
  width?: string | number;
  responsive?: boolean;
  onFileUpload?: (data: any, filename?: string) => void;
};

export function Hydroscope({
  data,
  height = 600,
  width = "100%",
}: HydroscopeProps) {
  const style: React.CSSProperties = {
    height: typeof height === "number" ? `${height}px` : height,
    width: typeof width === "number" ? `${width}px` : width,
    border: "1px solid #e5e7eb",
    borderRadius: 8,
    padding: 16,
    background: "#fafafa",
    fontFamily: "var(--ifm-font-family-base)",
  };

  return (
    <div style={style}>
      <h3 style={{ marginTop: 0 }}>Hydroscope (shim)</h3>
      <p style={{ color: "#6b7280", marginTop: 4 }}>
        The real @hydro-project/hydroscope package is not available during this
        build.
      </p>
      <pre
        style={{
          maxHeight: 300,
          overflow: "auto",
          background: "#fff",
          padding: 12,
        }}
      >
        {JSON.stringify(data ?? { nodes: [], edges: [] }, null, 2)}
      </pre>
    </div>
  );
}

export function enableResizeObserverErrorSuppression() {
  // no-op in shim
}

export function disableResizeObserverErrorSuppression() {
  // no-op in shim
}

// Basic implementation compatible with the docs page usage
export async function parseDataFromUrl(
  dataParam: string | null,
  compressedParam: string | null
): Promise<any | null> {
  if (!dataParam && !compressedParam) return null;

  try {
    const b64 = (compressedParam ?? dataParam ?? "").trim();
    if (!b64) return null;

    // Decode base64url without padding
    const normalized = b64.replace(/-/g, "+").replace(/_/g, "/");
    const pad = "=".repeat((4 - (normalized.length % 4)) % 4);
    const binary = atob(normalized + pad);
    const bytes = Uint8Array.from(binary, (c) => c.charCodeAt(0));

    // If compressed flag present, try to gunzip; otherwise treat as UTF-8 JSON
    if (compressedParam) {
      // In shim we don't include a gzip lib; try to parse as JSON fallback
      const text = new TextDecoder().decode(bytes);
      return JSON.parse(text);
    }

    const text = new TextDecoder().decode(bytes);
    return JSON.parse(text);
  } catch (e) {
    console.warn(
      "Hydroscope shim failed to parse URL data, returning null:",
      e
    );
    return null;
  }
}
