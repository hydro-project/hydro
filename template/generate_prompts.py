#!/usr/bin/env python3

"""
Script to generate prompt files from templates by filling in placeholders
with content from source files

Template syntax:
  {{INJECT_FILE:path/to/file}} - Injects the entire file content
  {{INJECT_RUSTDOC:path/to/file.rs:First line of doc comment}} - Extracts rustdoc comment
"""

import os
import re
import sys
from pathlib import Path

def extract_rustdoc(file_path, start_pattern):
    """Extract rustdoc comment starting with a specific pattern"""
    with open(file_path, 'r') as f:
        lines = f.readlines()
    
    result = []
    in_doc_block = False
    
    for line in lines:
        if not in_doc_block:
            # Look for the starting pattern
            if line.startswith('///') and start_pattern in line:
                in_doc_block = True
                # Remove the /// prefix and add to result
                result.append(line[4:] if line.startswith('/// ') else line[3:])
        else:
            # Continue collecting doc lines
            if line.startswith('///'):
                result.append(line[4:] if line.startswith('/// ') else line[3:])
            else:
                # End of doc block
                break
    
    return ''.join(result).rstrip()

def process_template(template_path, output_path, project_root):
    """Process a template file and generate output"""
    print(f"Processing template: {template_path}")
    print(f"Output: {output_path}")
    
    with open(template_path, 'r') as f:
        content = f.read()
    
    # Process INJECT_FILE placeholders
    for match in re.finditer(r'\{\{INJECT_FILE:([^}]+)\}\}', content):
        placeholder = match.group(0)
        file_path = match.group(1)
        full_path = project_root / file_path
        
        if full_path.exists():
            with open(full_path, 'r') as f:
                file_content = f.read()
            content = content.replace(placeholder, file_content)
        else:
            print(f"Warning: File not found: {full_path}")
    
    # Process INJECT_RUSTDOC placeholders
    for match in re.finditer(r'\{\{INJECT_RUSTDOC:([^:]+):([^}]+)\}\}', content):
        placeholder = match.group(0)
        file_path = match.group(1)
        pattern = match.group(2)
        full_path = project_root / file_path
        
        if full_path.exists():
            doc_content = extract_rustdoc(full_path, pattern)
            content = content.replace(placeholder, doc_content)
        else:
            print(f"Warning: File not found: {full_path}")
    
    # Write output
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, 'w') as f:
        f.write(content)
    
    print(f"Generated: {output_path}")

def main():
    # Get script directory and paths
    script_dir = Path(__file__).parent.resolve()
    template_dir = script_dir / "prompts"
    output_dir = script_dir / "hydro" / ".prompts"
    project_root = script_dir.parent
    
    # Create output directory
    output_dir.mkdir(parents=True, exist_ok=True)
    
    # Process all template files
    templates = list(template_dir.glob("*.template"))
    
    if not templates:
        print("No template files found in", template_dir)
        return 1
    
    for template_path in templates:
        # Get output filename (remove .template extension)
        output_filename = template_path.stem
        output_path = output_dir / output_filename
        
        process_template(template_path, output_path, project_root)
    
    print("\nAll prompt files generated successfully!")
    return 0

if __name__ == "__main__":
    sys.exit(main())
