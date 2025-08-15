/**
 * Validation Call Tracker
 * 
 * Collects stack traces from validation calls to understand:
 * 1. Where validation is being called from
 * 2. How frequently each call site triggers validation
 * 3. Which call paths are most common during testing
 */

interface ValidationCallSite {
  stack: string;
  count: number;
  lastCalled: Date;
  methods: string[]; // Extracted method names from stack
}

export class ValidationTracker {
  private static callSites = new Map<string, ValidationCallSite>();
  private static enabled = false;
  private static callGraph = new Map<string, Set<string>>(); // caller -> callees

  static enable() {
    this.enabled = true;
    this.callSites.clear();
    this.callGraph.clear();
  }

  static disable() {
    this.enabled = false;
  }

  static track(validationType: 'validateInvariants' | 'validateAllInvariants') {
    if (!this.enabled) return;

    const stack = new Error().stack || '';
    const stackLines = stack.split('\n').slice(2, 15); // Get more stack frames for better analysis
    
    // Enhanced parsing to extract more meaningful information
    const methods = stackLines
      .map(line => {
        // Match various stack trace formats
        let match = line.match(/at\s+(?:Object\.)?(\w+(?:\.\w+)?)/);
        if (!match) {
          match = line.match(/at\s+(\w+(?:\.\w+)?)\s+\(/);
        }
        if (!match) {
          match = line.match(/at\s+([^\s]+)/);
        }
        
        if (match) {
          let methodName = match[1];
          // Clean up common patterns
          methodName = methodName.replace(/^Object\./, '');
          methodName = methodName.replace(/^\$/, '');
          
          // Extract file context if available
          const fileMatch = line.match(/\(([^/\\]*\.(?:ts|js)):\d+:\d+\)/);
          if (fileMatch) {
            const fileName = fileMatch[1].replace(/\.(ts|js)$/, '');
            return `${methodName}@${fileName}`;
          }
          
          return methodName;
        }
        return null;
      })
      .filter(method => method !== null && method !== 'unknown') as string[];

    // Build call graph relationships
    for (let i = 0; i < methods.length - 1; i++) {
      const caller = methods[i];
      const callee = methods[i + 1];
      
      if (!this.callGraph.has(caller)) {
        this.callGraph.set(caller, new Set());
      }
      this.callGraph.get(caller)!.add(callee);
    }

    // Create a more detailed call site key
    const topMethods = methods.slice(0, 5);
    const callSiteKey = topMethods.join(' → ');
    
    const existing = this.callSites.get(callSiteKey);
    if (existing) {
      existing.count++;
      existing.lastCalled = new Date();
    } else {
      this.callSites.set(callSiteKey, {
        stack: stackLines.join('\n'),
        count: 1,
        lastCalled: new Date(),
        methods: topMethods
      });
    }
  }

  static getReport(): {
    totalCalls: number;
    uniqueCallSites: number;
    topCallSites: Array<{
      methods: string[];
      count: number;
      percentage: number;
      stack: string;
    }>;
    callGraph: Map<string, Set<string>>;
    hotPaths: Array<{
      path: string;
      frequency: number;
      percentage: number;
    }>;
  } {
    const totalCalls = Array.from(this.callSites.values()).reduce((sum, site) => sum + site.count, 0);
    const sortedSites = Array.from(this.callSites.values())
      .sort((a, b) => b.count - a.count)
      .slice(0, 15); // Top 15 call sites

    // Analyze hot paths by looking at call chains
    const pathFrequency = new Map<string, number>();
    for (const [path, site] of this.callSites) {
      pathFrequency.set(path, site.count);
    }

    const hotPaths = Array.from(pathFrequency.entries())
      .map(([path, frequency]) => ({
        path,
        frequency,
        percentage: Math.round((frequency / totalCalls) * 100)
      }))
      .sort((a, b) => b.frequency - a.frequency)
      .slice(0, 10);

    return {
      totalCalls,
      uniqueCallSites: this.callSites.size,
      topCallSites: sortedSites.map(site => ({
        methods: site.methods,
        count: site.count,
        percentage: Math.round((site.count / totalCalls) * 100),
        stack: site.stack
      })),
      callGraph: this.callGraph,
      hotPaths
    };
  }

  static printReport() {
    const report = this.getReport();
    
    console.log('\n🔍 VALIDATION CALL STACK ANALYSIS');
    console.log('====================================');
    console.log(`📊 Total validation calls: ${report.totalCalls}`);
    console.log(`🔗 Unique call sites: ${report.uniqueCallSites}`);
    console.log(`🌐 Call graph nodes: ${report.callGraph.size}`);
    
    if (report.totalCalls === 0) {
      console.log('❌ No validation calls captured - tracking may not be working');
      return;
    }
    
    console.log('\n� TOP VALIDATION CALL SITES:');
    console.log('================================');
    
    report.topCallSites.forEach((site, index) => {
      console.log(`\n${index + 1}. 📈 ${site.count} calls (${site.percentage}%)`);
      console.log(`   🔗 Call chain: ${site.methods.slice(0, 4).join(' → ')}`);
      
      if (site.stack) {
        const relevantLines = site.stack.split('\n').slice(0, 3);
        relevantLines.forEach((line, i) => {
          if (line.trim()) {
            const cleanLine = line.replace(/^\s*at\s+/, '').trim();
            console.log(`   ${i === 0 ? '📍' : '  '} ${cleanLine}`);
          }
        });
      }
    });

    console.log('\n🛤️  HOT PATHS ANALYSIS:');
    console.log('========================');
    
    report.hotPaths.forEach((hotPath, index) => {
      console.log(`\n${index + 1}. 🔥 ${hotPath.frequency} calls (${hotPath.percentage}%)`);
      console.log(`   ➡️  ${hotPath.path}`);
    });

    console.log('\n🕸️  CALL GRAPH ANALYSIS:');
    console.log('=========================');
    
    // Find most connected nodes in call graph
    const nodeConnections = new Map<string, number>();
    for (const [caller, callees] of report.callGraph) {
      nodeConnections.set(caller, (nodeConnections.get(caller) || 0) + callees.size);
      for (const callee of callees) {
        nodeConnections.set(callee, (nodeConnections.get(callee) || 0) + 1);
      }
    }
    
    const topNodes = Array.from(nodeConnections.entries())
      .sort((a, b) => b[1] - a[1])
      .slice(0, 8);
      
    console.log('📊 Most connected functions:');
    topNodes.forEach(([node, connections], index) => {
      const callees = report.callGraph.get(node);
      console.log(`   ${index + 1}. ${node} (${connections} connections)`);
      if (callees && callees.size > 0) {
        const topCallees = Array.from(callees).slice(0, 3);
        console.log(`      ➡️  calls: ${topCallees.join(', ')}${callees.size > 3 ? '...' : ''}`);
      }
    });
    
    console.log('\n💡 RECOMMENDATIONS:');
    console.log('=====================');
    const topPercentage = report.topCallSites[0]?.percentage || 0;
    if (topPercentage > 40) {
      console.log(`🔥 HIGH CONCENTRATION: ${report.topCallSites[0].methods[0]} accounts for ${topPercentage}% of all validation calls`);
      console.log(`   → Consider reducing validation frequency in this path`);
    }
    if (report.uniqueCallSites > 25) {
      console.log(`🌊 HIGH CALL SITE DIVERSITY: ${report.uniqueCallSites} different locations call validation`);
      console.log(`   → Consider consolidating validation calls`);
    }
    if (report.totalCalls > 50) {
      console.log(`⚡ HIGH FREQUENCY: ${report.totalCalls} total validation calls detected`);
      console.log(`   → Consider batch validation or validation suppression`);
    }
    
    // Analyze redundancy
    const potentialRedundancy = report.topCallSites.filter(site => 
      site.methods.some(method => method.includes('setTimeout') || method.includes('async'))
    );
    if (potentialRedundancy.length > 0) {
      console.log(`🔄 POTENTIAL REDUNDANCY: ${potentialRedundancy.length} async validation patterns detected`);
      console.log(`   → Check for duplicate validations in async callbacks`);
    }
  }

  static reset() {
    this.callSites.clear();
  }
}
