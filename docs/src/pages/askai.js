import React, { useEffect } from "react";
import { useLocation } from "@docusaurus/router";
import Layout from "@theme/Layout";

import styles from "./askai.module.css";

export default function AskAI() {
  const location = useLocation(); // Track current route

  useEffect(() => {
    const scriptId = "runllm-widget-script";

    // Check if script is already added
    if (!document.getElementById(scriptId)) {
      const script = document.createElement("script");
      script.type = "module";
      script.id = scriptId;
      script.src = "https://widget.runllm.com";

      script.setAttribute("version", "stable");
      script.setAttribute("runllm-keyboard-shortcut", "Mod+j");
      script.setAttribute("runllm-name", "Hydro");
      script.setAttribute("runllm-position", "TOP_RIGHT");
      script.setAttribute("runllm-assistant-id", "600");
      script.setAttribute("runllm-preset", "docusaurus");
      script.setAttribute("runllm-brand-logo", "img/hydro-turtle.png");

      script.async = true;
      document.body.appendChild(script);
    }

    const removeWidget = () => {
      // Remove script
      const existingScript = document.getElementById(scriptId);
      if (existingScript) {
        existingScript.remove();
      }

      // Remove widget and any associated elements
      const removeElements = () => {
        document
          .querySelectorAll("runllm-widget, iframe[src*='widget.runllm.com'], .runllm-container")
          .forEach((el) => el.remove());
      };

      // Try removing immediately and again after a short delay
      removeElements();
      setTimeout(removeElements, 1000);
    };

    return () => {
      removeWidget();
    };
  }, [location.pathname]); // Runs effect on every route change

  return (
    <Layout description="AI assistant for the Hydro API">
      <main>
        <div className={styles["main_text"]}>  
          <h2> Hydro AI Assistant </h2>
          <p>
            Hydro has a built-in AI assistant that can help you with your
            questions about the Hydro API. You can ask it anything related to
            Hydro, and it will try its best to answer you.
          </p>
          <p>To give it a try, just press the "Ask AI" button.</p>
        </div>
      </main>
    </Layout>
  );
}
