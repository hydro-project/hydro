/**
 * Suppress ResizeObserver errors globally
 * This script must load before any other scripts to catch errors from webpack-dev-server
 */
(function() {
  const resizeObserverLoopErr = 'ResizeObserver loop';
  
  // Suppress via window.onerror
  const originalOnError = window.onerror;
  window.onerror = function(msg, url, lineNo, columnNo, error) {
    const errorMsg = String(msg || error || '');
    if (errorMsg.includes(resizeObserverLoopErr)) {
      console.debug('[Hydroscope] Suppressed ResizeObserver error');
      return true; // Suppress
    }
    return originalOnError ? originalOnError(msg, url, lineNo, columnNo, error) : false;
  };
  
  // Suppress via error event listener (capture phase for webpack-dev-server)
  window.addEventListener('error', function(e) {
    const errorMsg = String(e.message || e.error || '');
    if (errorMsg.includes(resizeObserverLoopErr)) {
      console.debug('[Hydroscope] Suppressed ResizeObserver error via event listener');
      e.stopImmediatePropagation();
      e.preventDefault();
      return false;
    }
  }, true);
  
  // Suppress unhandled rejections
  window.addEventListener('unhandledrejection', function(e) {
    const errorMsg = String(e.reason || '');
    if (errorMsg.includes(resizeObserverLoopErr)) {
      console.debug('[Hydroscope] Suppressed ResizeObserver unhandled rejection');
      e.stopImmediatePropagation();
      e.preventDefault();
      return false;
    }
  }, true);
  
  console.debug('[Hydroscope] ResizeObserver error suppression installed');
})();
