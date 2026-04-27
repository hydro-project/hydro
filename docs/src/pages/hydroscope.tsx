import React, { useState, useEffect } from "react";
import Layout from "@theme/Layout";

export default function HydroscopePage() {
  const [HydroscopeComponent, setHydroscopeComponent] = useState<any>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (typeof window === "undefined") return;

    // Suppress ResizeObserver loop errors only while this page is mounted
    const onError = (e: ErrorEvent) => {
      if (e.message?.includes("ResizeObserver")) { e.stopImmediatePropagation(); e.stopPropagation(); e.preventDefault(); }
    };
    window.addEventListener("error", onError, true);

    let origRO: typeof ResizeObserver | undefined;
    if (window.ResizeObserver) {
      origRO = window.ResizeObserver;
      const Orig = origRO;
      window.ResizeObserver = class extends Orig {
        constructor(cb: ResizeObserverCallback) {
          let frameId: number | null = null;
          let latest: { entries: ResizeObserverEntry[]; observer: ResizeObserver } | null = null;
          super((entries, observer) => {
            latest = { entries, observer };
            if (frameId !== null) return;
            frameId = requestAnimationFrame(() => {
              frameId = null;
              if (latest) cb(latest.entries, latest.observer);
              latest = null;
            });
          });
        }
      };
    }

    (async () => {
      try {
        await import("@xyflow/react/dist/style.css");
        await import("@hydro-project/hydroscope/style.css");
        const mod = await import("@hydro-project/hydroscope");
        setHydroscopeComponent(() => mod.Hydroscope);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to load Hydroscope");
      }
    })();

    return () => {
      window.removeEventListener("error", onError, true);
      if (origRO) window.ResizeObserver = origRO;
    };
  }, []);

  if (error) {
    return (
      <Layout title="Hydroscope" description="Interactive graph visualization">
        <div style={{ padding: "40px", textAlign: "center", color: "#d32f2f" }}>
          <h3>Error</h3>
          <p>{error}</p>
          <button onClick={() => window.location.reload()}
            style={{ padding: "8px 16px", background: "#1976d2", color: "white", border: "none", borderRadius: 4, cursor: "pointer" }}>
            Retry
          </button>
        </div>
      </Layout>
    );
  }

  if (!HydroscopeComponent) {
    return (
      <Layout title="Hydroscope" description="Interactive graph visualization">
        <div style={{ padding: "40px", textAlign: "center" }}>Loading Hydroscope…</div>
      </Layout>
    );
  }

  return (
    <Layout title="Hydroscope" description="Interactive graph visualization" noFooter={true}>
      <div style={{ height: "calc(100vh - var(--ifm-navbar-height, 60px))", overflow: "hidden" }}>
        <HydroscopeComponent height="100%" width="100%" responsive />
      </div>
    </Layout>
  );
}
