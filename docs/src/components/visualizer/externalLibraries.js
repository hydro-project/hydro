/**
 * External Library Loader for ELK (ReactFlow now handled via npm)
 * 
 * Manages loading of ELK.js library from CDN
 * ReactFlow v12 is now imported directly via npm
 */

let ELK = null;

export const loadExternalLibraries = async () => {
  // Load ELK.js for advanced layouts
  if (!window.ELK) {
    const elkScript = document.createElement('script');
    elkScript.src = 'https://unpkg.com/elkjs@0.8.2/lib/elk.bundled.js';
    document.head.appendChild(elkScript);
    
    await new Promise((resolve, reject) => {
      elkScript.onload = resolve;
      elkScript.onerror = reject;
    });
    
    ELK = new window.ELK();
  } else {
    ELK = new window.ELK();
  }
  
  return { ELK };
};

export { ELK };
