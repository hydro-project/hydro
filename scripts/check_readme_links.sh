#!/bin/bash
# Check that links in README.md pointing to hydro.run are valid
# by verifying they exist in the local website build

set -e

README_FILE="README.md"
BUILD_DIR="docs/build"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

if [ ! -f "$README_FILE" ]; then
    echo -e "${RED}ERROR: $README_FILE not found${NC}"
    exit 1
fi

if [ ! -d "$BUILD_DIR" ]; then
    echo -e "${RED}ERROR: $BUILD_DIR not found. Please build the website first.${NC}"
    exit 1
fi

echo "Checking README links against local website build..."
echo

# Extract all hydro.run URLs from README
# Pattern matches URLs until we hit a quote, paren, or whitespace delimiter
urls=$(grep -o 'https://hydro.run[^")]*' "$README_FILE" | sort -u)

if [ -z "$urls" ]; then
    echo -e "${YELLOW}WARNING: No hydro.run links found in README${NC}"
    exit 0
fi

failed=0
checked=0

while IFS= read -r url; do
    checked=$((checked + 1))
    
    # Extract path from URL (remove https://hydro.run)
    path="${url#https://hydro.run}"
    
    # If path is empty, it's the root
    if [ -z "$path" ]; then
        path="/"
    fi
    
    # Determine the file to check
    if [ "$path" = "/" ]; then
        # Root path
        file_to_check="$BUILD_DIR/index.html"
    elif [[ "$path" == *.pdf ]] || [[ "$path" == *.png ]] || [[ "$path" == *.jpg ]] || [[ "$path" == *.svg ]]; then
        # Static file (PDF, image, etc.)
        file_to_check="$BUILD_DIR$path"
    elif [[ "$path" == */ ]]; then
        # Path ends with slash, it's a directory/page
        file_to_check="$BUILD_DIR${path}index.html"
    else
        # Path without trailing slash - try multiple patterns to handle different URL conventions
        # Docusaurus can serve /docs/hydro from either /docs/hydro/index.html or /docs/hydro.html
        if [ -f "$BUILD_DIR${path}/index.html" ]; then
            file_to_check="$BUILD_DIR${path}/index.html"
        elif [ -f "$BUILD_DIR${path}.html" ]; then
            file_to_check="$BUILD_DIR${path}.html"
        elif [ -f "$BUILD_DIR${path}" ]; then
            file_to_check="$BUILD_DIR${path}"
        else
            # Default to index.html pattern (will fail validation if not found)
            file_to_check="$BUILD_DIR${path}/index.html"
        fi
    fi
    
    if [ -f "$file_to_check" ]; then
        echo -e "${GREEN}✓${NC} $url"
    else
        echo -e "${RED}✗${NC} $url"
        echo -e "  Expected file: $file_to_check"
        failed=$((failed + 1))
    fi
done <<< "$urls"

echo
echo "Checked $checked links"

if [ $failed -gt 0 ]; then
    echo -e "${RED}ERROR: $failed link(s) failed validation${NC}"
    exit 1
else
    echo -e "${GREEN}All links valid!${NC}"
    exit 0
fi
