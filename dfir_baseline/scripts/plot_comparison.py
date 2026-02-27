#!/usr/bin/env python3
"""
Plot 6-way comparison of server implementations under metastability conditions.
"""

import json
import sys
from pathlib import Path
from collections import defaultdict
import matplotlib.pyplot as plt

def load_metrics(metrics_file):
    events = []
    if not Path(metrics_file).exists():
        print(f"  Warning: {metrics_file} not found")
        return events
    with open(metrics_file, 'r') as f:
        for line in f:
            if line.strip():
                events.append(json.loads(line))
    return events

def analyze_metrics(events):
    if not events:
        return None
    
    # Track request send times and final outcomes
    request_send_times = {}  # req_id -> send_timestamp (seconds)
    request_final_outcome = {}  # req_id -> (outcome_type, latency_ms or None)
    retries_per_window = defaultdict(int)
    latencies_per_window = defaultdict(list)
    
    # First pass: record send times and retries
    for event in events:
        if event['type'] == 'request_sent':
            request_send_times[event['req_id']] = int(event['timestamp'] / 1000.0)
        elif event['type'] == 'request_retried':
            ts = int(event['timestamp'] / 1000.0)
            retries_per_window[ts] += 1
    
    # Second pass: determine final outcome for each request
    # Later events overwrite earlier ones, so we get the final state
    for event in events:
        req_id = event.get('req_id')
        if req_id is None:
            continue
        t = event['type']
        if t == 'response_received':
            request_final_outcome[req_id] = ('received', event['latency_ms'])
        elif t == 'request_rejected':
            # Only set if not already received (rejection can be followed by success on retry)
            if req_id not in request_final_outcome or request_final_outcome[req_id][0] != 'received':
                request_final_outcome[req_id] = ('rejected', None)
        elif t == 'request_timeout':
            if req_id not in request_final_outcome or request_final_outcome[req_id][0] not in ('received', 'rejected'):
                request_final_outcome[req_id] = ('timeout', None)
        elif t == 'request_failed':
            # Final failure after max retries
            request_final_outcome[req_id] = ('failed', None)
    
    # Build windows from send times and final outcomes
    windows = defaultdict(lambda: {'sent': 0, 'retried': 0, 'received': 0, 'timeout': 0, 'failed': 0, 'rejected': 0, 'latencies': []})
    
    for req_id, send_ts in request_send_times.items():
        windows[send_ts]['sent'] += 1
        if req_id in request_final_outcome:
            outcome, latency = request_final_outcome[req_id]
            if outcome == 'received':
                windows[send_ts]['received'] += 1
                if latency is not None:
                    windows[send_ts]['latencies'].append(latency)
            elif outcome == 'rejected':
                windows[send_ts]['rejected'] += 1
            elif outcome == 'timeout':
                windows[send_ts]['timeout'] += 1
            elif outcome == 'failed':
                windows[send_ts]['failed'] += 1
    
    # Add retries to windows
    for ts, count in retries_per_window.items():
        windows[ts]['retried'] = count
    
    if not windows:
        return None
    
    timestamps = sorted(windows.keys())
    start = timestamps[0]
    
    ts_data = {'time': [], 'original_rate': [], 'total_rate': [], 'success_rate': [], 'timeout_rate': [], 'p50': [], 'p99': []}
    
    for t in timestamps:
        w = windows[t]
        ts_data['time'].append(t - start)
        ts_data['original_rate'].append(w['sent'])
        ts_data['total_rate'].append(w['sent'] + w['retried'])
        
        # Success = received. Rejected/timeout/failed are failures.
        total = w['sent']
        if total > 0:
            success = (w['received'] / total) * 100.0
            timeout = (w['timeout'] / total) * 100.0
        else:
            success, timeout = 0.0, 0.0
        ts_data['success_rate'].append(success)
        ts_data['timeout_rate'].append(timeout)
        
        if w['latencies']:
            s = sorted(w['latencies'])
            ts_data['p50'].append(s[int(len(s) * 0.5)])
            ts_data['p99'].append(s[min(int(len(s) * 0.99), len(s) - 1)])
        else:
            ts_data['p50'].append(0)
            ts_data['p99'].append(0)
    
    return ts_data

def plot_6way(datasets, output_file):
    """Plot 6 server implementations side by side."""
    n = len(datasets)
    fig, axes = plt.subplots(3, n, figsize=(4*n, 10), sharey='row')
    fig.suptitle('DFIR Metastability: 7-Way Server Comparison', fontsize=14, fontweight='bold')
    
    baseline_end, trigger_end = 30, 45
    
    # Show tick labels on all subplots (sharey hides them by default)
    for row in range(3):
        for col in range(n):
            axes[row, col].tick_params(labelleft=True)
    
    for col, (ts, title) in enumerate(datasets):
        if ts is None:
            for row in range(3):
                axes[row, col].text(0.5, 0.5, 'No Data', ha='center', va='center', fontsize=12)
                axes[row, col].set_title(title if row == 0 else '')
            continue
        
        time = ts['time']
        max_t = max(time) if time else 135
        
        # Offered rate - show both original and total (with retries)
        ax = axes[0, col]
        ax.plot(time, ts['original_rate'], 'b-', lw=1.5, label='Original')
        ax.plot(time, ts['total_rate'], 'r--', lw=1.5, alpha=0.7, label='+ Retries')
        ax.axvspan(0, baseline_end, alpha=0.1, color='green')
        ax.axvspan(baseline_end, trigger_end, alpha=0.1, color='red')
        ax.axvspan(trigger_end, max_t, alpha=0.1, color='blue')
        ax.set_ylabel('Req/s')
        ax.set_title(f'{title}\nOffered Load', fontsize=10)
        if col == 0:
            ax.legend(loc='upper right', fontsize=7)
        ax.grid(True, alpha=0.3)
        ax.set_xlim(0, max_t)
        
        # Success rate
        ax = axes[1, col]
        ax.plot(time, ts['success_rate'], 'g-', lw=1.5, label='Success')
        ax.plot(time, ts['timeout_rate'], 'r--', lw=1.5, label='Timeout')
        ax.axhline(y=100, color='gray', ls=':', alpha=0.5)
        ax.axvspan(0, baseline_end, alpha=0.1, color='green')
        ax.axvspan(baseline_end, trigger_end, alpha=0.1, color='red')
        ax.axvspan(trigger_end, max_t, alpha=0.1, color='blue')
        ax.set_ylabel('%')
        ax.set_title('Success Rate', fontsize=10)
        if col == n - 1:
            ax.legend(loc='lower right', fontsize=7)
        ax.grid(True, alpha=0.3)
        ax.set_ylim(-5, 105)
        ax.set_xlim(0, max_t)
        
        # Latency
        ax = axes[2, col]
        ax.plot(time, ts['p50'], 'b-', lw=1.5, label='p50')
        ax.plot(time, ts['p99'], 'r-', lw=1.5, label='p99')
        ax.axvspan(0, baseline_end, alpha=0.1, color='green')
        ax.axvspan(baseline_end, trigger_end, alpha=0.1, color='red')
        ax.axvspan(trigger_end, max_t, alpha=0.1, color='blue')
        ax.set_xlabel('Time (s)')
        ax.set_ylabel('Latency (ms)')
        ax.set_title('Latency', fontsize=10)
        if col == n - 1:
            ax.legend(loc='upper right', fontsize=7)
        ax.grid(True, alpha=0.3)
        ax.set_xlim(0, max_t)
    
    from matplotlib.patches import Patch
    legend_elements = [
        Patch(facecolor='green', alpha=0.3, label='Baseline (0-30s)'),
        Patch(facecolor='red', alpha=0.3, label='3x Burst (30-45s)'),
        Patch(facecolor='blue', alpha=0.3, label='Recovery (45s+)')
    ]
    fig.legend(handles=legend_elements, loc='lower center', ncol=3, fontsize=10)
    
    plt.tight_layout(rect=[0, 0.05, 1, 0.95])
    plt.savefig(output_file, dpi=150, bbox_inches='tight')
    print(f"Saved: {output_file}")
    
    # Summary
    print("\n" + "="*70)
    print("Recovery Phase Analysis (45s+):")
    print("-"*70)
    for ts, name in datasets:
        if ts is None:
            print(f"  {name}: NO DATA")
            continue
        recovery = [i for i, t in enumerate(ts['time']) if t > trigger_end]
        if recovery:
            avg_success = sum(ts['success_rate'][i] for i in recovery) / len(recovery)
            status = "✓ RECOVERED" if avg_success > 80 else "✗ COLLAPSED"
            print(f"  {name}: {avg_success:.1f}% success {status}")
        else:
            print(f"  {name}: No recovery data")
    print("="*70)

def main():
    base = "/tmp/dfir_comparison"
    
    files = [
        (f"{base}/dfir_multistage.jsonl", "DFIR\nMulti-Stage"),
        (f"{base}/dfir_unbounded.jsonl", "DFIR\nSingle Unbnd"),
        (f"{base}/tcp_blocking.jsonl", "TCP\nBlocking"),
        (f"{base}/single_admission_polite.jsonl", "Single+AC\n(polite)"),
        (f"{base}/single_admission_rude.jsonl", "Single+AC\n(rude)"),
        (f"{base}/multi_admission_polite.jsonl", "Multi+AC\n(polite)"),
        (f"{base}/multi_admission_rude.jsonl", "Multi+AC\n(rude)"),
    ]
    
    output_file = "/tmp/dfir_metastability_comparison.png"
    
    print("Loading metrics...")
    datasets = []
    for path, name in files:
        events = load_metrics(path)
        print(f"  {name.replace(chr(10), ' ')}: {len(events)} events")
        ts = analyze_metrics(events)
        datasets.append((ts, name))
    
    plot_6way(datasets, output_file)

if __name__ == '__main__':
    main()
