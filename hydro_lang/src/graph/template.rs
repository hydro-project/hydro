//! Minimal HTML template for legacy in-browser JSON visualization.
//! The placeholder `{{GRAPH_DATA}}` will be replaced with the JSON string.

/// Return a simple HTML page embedding the graph JSON in a <script> tag.
/// This is intended for debugging only and may be deprecated in the future.
pub fn get_template() -> String {
    // Keep this tiny and self-contained; consumers just replace the placeholder and open the file.
    let html = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Hydroscope (Legacy HTML)</title>
    <style>
      html, body { margin: 0; padding: 0; height: 100%; font-family: -apple-system, BlinkMacSystemFont, Segoe UI, Roboto, Oxygen, Ubuntu, Cantarell, 'Fira Sans', 'Droid Sans', 'Helvetica Neue', Arial, sans-serif; }
      #app { height: 100%; display: grid; grid-template-rows: 48px 1fr; }
      header { display: flex; align-items: center; padding: 0 16px; border-bottom: 1px solid #e5e7eb; background: #fafafa; }
      header h1 { font-size: 16px; margin: 0; font-weight: 600; }
      pre { margin: 0; padding: 16px; background: #0b1021; color: #d1d5db; overflow: auto; }
      .hint { padding: 12px 16px; font-size: 13px; color: #374151; background: #f9fafb; border-bottom: 1px solid #e5e7eb; }
    </style>
  </head>
  <body>
    <div id="app">
      <header><h1>Hydroscope (Legacy HTML)</h1></header>
      <div class="hint">This legacy viewer only prints the JSON payload. Use the docs visualizer for the full experience.</div>
      <pre id="json"></pre>
    </div>
    <script>
      const data = {{GRAPH_DATA}};
      const el = document.getElementById('json');
      try {
        el.textContent = JSON.stringify(data, null, 2);
      } catch (_) {
        el.textContent = String(data);
      }
    </script>
  </body>
</html>
"#;
    html.to_string()
}
