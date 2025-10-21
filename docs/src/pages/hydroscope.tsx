import React, { useState, useEffect } from "react";
import Layout from "@theme/Layout";
import BrowserOnly from "@docusaurus/BrowserOnly";
import "@xyflow/react/dist/style.css";

import { Hydroscope, parseDataFromUrl } from "@hydro-project/hydroscope";

import {
  enableResizeObserverErrorSuppression,
  disableResizeObserverErrorSuppression,
} from "@hydro-project/hydroscope";

function HydroscopePage() {
  const [urlData, setUrlData] = useState<any>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Initialize ResizeObserver error suppression and parse URL parameters
  useEffect(() => {
    // Enable ResizeObserver error suppression for Docusaurus environment
    enableResizeObserverErrorSuppression();

    const parseUrlData = async () => {
      try {
        const urlParams = new URLSearchParams(window.location.search);
        const dataParam = urlParams.get("data");
        const compressedParam = urlParams.get("compressed");

        if (dataParam || compressedParam) {
          const parsedData = await parseDataFromUrl(dataParam, compressedParam);
          if (parsedData) {
            setUrlData(parsedData);
          }
        }
      } catch (err) {
        console.error("❌ Failed to parse URL data:", err);
        setError(`Failed to parse URL data: ${err.message}`);
      } finally {
        setLoading(false);
      }
    };

    parseUrlData();

    // Cleanup on unmount
    return () => {
      disableResizeObserverErrorSuppression();
    };
  }, []);

  // Calculate dynamic height for responsive behavior
  const [height, setHeight] = useState("600px");

  useEffect(() => {
    const calculateHeight = () => {
      try {
        const navbar = document.querySelector(".navbar") as HTMLElement;
        const navbarHeight = navbar ? navbar.offsetHeight : 60;
        setHeight(`calc(100vh - ${navbarHeight + 40}px)`); // 40px for padding
      } catch (error) {
        setHeight("600px"); // fallback
      }
    };

    calculateHeight();
    window.addEventListener("resize", calculateHeight);
    return () => window.removeEventListener("resize", calculateHeight);
  }, []);

  if (loading) {
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
      <div style={{ height, overflow: "hidden" }}>
        <Hydroscope
          data={urlData} // Pass URL data if available
          height={height}
          width="100%"
          responsive={true} // Enable responsive height calculation
          // All other props use their defaults:
          // showControls, showMiniMap, showBackground, showFileUpload,
          // showInfoPanel, showStylePanel, enableCollapse all default to true
          // initialLayoutAlgorithm defaults to mrtree
          // initialColorPalette defaults to Set3
          onFileUpload={(data, filename) => {
            // Update the data state so the component shows the visualization
            setUrlData(data);
          }}
        />
      </div>
    </Layout>
  );
}

// Main export with BrowserOnly wrapper
export default function HydroscopePageWrapper() {
  return (
    <BrowserOnly fallback={<div>Loading...</div>}>
      {() => <HydroscopePage />}
    </BrowserOnly>
  );
}
