import React, { useState, useEffect } from "react";
import Layout from "@theme/Layout";

export default function HydroscopePage() {
  const [HydroscopeComponent, setHydroscopeComponent] = useState<any>(null);
  const [urlData, setUrlData] = useState<any>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Dynamically import Hydroscope library on mount (browser only)
  useEffect(() => {
    // Only run in browser
    if (typeof window === "undefined") {
      return;
    }

    const loadHydroscope = async () => {
      try {
        // Import CSS first
        await import("@xyflow/react/dist/style.css");

        // Then import the library
        const hydroscopeModule = await import("@hydro-project/hydroscope");
        setHydroscopeComponent(() => hydroscopeModule.Hydroscope);
      } catch (err) {
        console.error("❌ Failed to load Hydroscope:", err);
        setError("Failed to load Hydroscope library");
        setLoading(false);
        return;
      }

      // Parse URL parameters after library is loaded
      try {
        const searchParams = new URLSearchParams(window.location.search);
        let dataParam = searchParams.get("data");
        let compressedParam = searchParams.get("compressed");

        // Fallback: also support hash fragment params (#data= / #compressed=)
        const hash = window.location.hash?.replace(/^#/, "");
        if (hash && !dataParam && !compressedParam) {
          const hashParams = new URLSearchParams(hash);
          dataParam = hashParams.get("data") || dataParam;
          compressedParam = hashParams.get("compressed") || compressedParam;
        }

        if (dataParam || compressedParam) {
          const hydroscopeModule = await import("@hydro-project/hydroscope");
          const parsedData = await hydroscopeModule.parseDataFromUrl(
            dataParam,
            compressedParam
          );
          if (parsedData) {
            setUrlData(parsedData);
          }
        }
      } catch (err) {
        console.error("❌ Failed to parse URL data:", err);
        const errorMessage =
          err instanceof Error
            ? err.message
            : typeof err === "string"
              ? err
              : "Unknown error";
        setError(`Failed to parse URL data: ${errorMessage}`);
      } finally {
        setLoading(false);
      }
    };

    loadHydroscope();
  }, []);

  if (loading || !HydroscopeComponent) {
    return (
      <Layout title="Hydroscope" description="Interactive graph visualization">
        <div style={{ padding: "40px", textAlign: "center" }}>
          <p>Loading Hydroscope...</p>
        </div>
      </Layout>
    );
  }

  if (error) {
    return (
      <Layout title="Hydroscope" description="Interactive graph visualization">
        <div style={{ padding: "40px", textAlign: "center", color: "#d32f2f" }}>
          <h3>Error Loading Hydroscope</h3>
          <p>{error}</p>
          <button
            onClick={() => {
              try {
                window.location.reload();
              } catch (err) {
                // Fallback for test environments where reload isn't available
                window.location.replace(window.location.href);
              }
            }}
            style={{
              padding: "10px 20px",
              backgroundColor: "#1976d2",
              color: "white",
              border: "none",
              borderRadius: "4px",
              cursor: "pointer",
            }}
          >
            Retry
          </button>
        </div>
      </Layout>
    );
  }

  return (
    <Layout
      title="Hydroscope"
      description="Interactive graph visualization"
      noFooter={true}
    >
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          height: "calc(100vh - var(--ifm-navbar-height, 60px))",
          overflow: "hidden",
        }}
      >
        <HydroscopeComponent
          data={urlData} // Pass URL data if available
          height="100%"
          width="100%"
          responsive={true} // Enable responsive height calculation
          // All other props use their defaults:
          // showControls, showMiniMap, showBackground, showFileUpload,
          // showInfoPanel, showStylePanel, enableCollapse all default to true
          // initialLayoutAlgorithm defaults to mrtree
          // initialColorPalette defaults to Set3
          onFileUpload={(data) => {
            // Update the data state so the component shows the visualization
            setUrlData(data);
          }}
        />
      </div>
    </Layout>
  );
}
