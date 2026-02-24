#!/bin/bash
# Test script for Brain Server v6.2 - Semantic Vector Search

set -e

BASE_URL="http://127.0.0.1:8765"

echo "🧠 Brain Server v6.2 API Tests"
echo "================================"
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

# Test function
test_api() {
    local name="$1"
    local method="$2"
    local endpoint="$3"
    local data="$4"
    local expected="$5"
    
    echo -n "Testing: $name ... "
    
    if [ "$method" = "GET" ]; then
        response=$(curl -s -X GET "$BASE_URL$endpoint")
    else
        response=$(curl -s -X POST "$BASE_URL$endpoint" \
            -H "Content-Type: application/json" \
            -d "$data")
    fi
    
    if echo "$response" | grep -q "$expected"; then
        echo -e "${GREEN}✓ PASS${NC}"
        ((TESTS_PASSED++))
    else
        echo -e "${RED}✗ FAIL${NC}"
        echo "   Expected: $expected"
        echo "   Got: $response"
        ((TESTS_FAILED++))
    fi
}

echo "1. Health Check"
test_api "Health endpoint" "GET" "/health" "" '"status":"ok"'

echo ""
echo "2. Database Stats"
test_api "Stats endpoint" "GET" "/stats" "" '"count"'

echo ""
echo "3. Add Chunks"
echo "   Adding goat farming knowledge..."
test_api "Add goat housing" "POST" "/add" \
    '{"text":"Goats need 15-20 sq ft of shelter space per animal. Minimum 4ft fencing to prevent escape. They are social animals - never keep just one.","title":"Goat Housing Basics"}' \
    '"success":true'

echo "   Adding tiling knowledge..."
test_api "Add tiling procedure" "POST" "/add" \
    '{"text":"For bathroom floor: apply waterproof primer before tiling. Use 3mm notched trowel for small tiles, 6mm for large. Grout after 24 hours cure time.","title":"Bathroom Tiling Procedure"}' \
    '"success":true'

echo "   Adding sleep science..."
test_api "Add sleep science" "POST" "/add" \
    '{"text":"The gut microbiome produces sleep-regulating compounds like serotonin and melatonin. Antibiotic use can disrupt sleep patterns for weeks.","title":"Gut-Brain Sleep Connection"}' \
    '"success":true'

echo ""
echo "4. Semantic Search"
echo "   Testing goat housing query..."
test_api "Search for goat space" "POST" "/search" \
    '{"query":"How much space do goats need?","k":5}' \
    '"success":true'

echo "   Testing tiling query..."
test_api "Search for tiling prep" "POST" "/search" \
    '{"query":"What trowel size for bathroom tiles?","k":5}' \
    '"success":true'

echo "   Testing gut-sleep connection..."
test_api "Search for gut sleep" "POST" "/search" \
    '{"query":"How does gut health affect sleep?","k":5}' \
    '"success":true'

echo ""
echo "5. Matryoshka Truncation"
echo "   Testing with 256 dims..."
test_api "Search with truncation" "POST" "/search" \
    '{"query":"goat housing requirements","k":5,"truncate_dims":256}' \
    '"success":true'

echo ""
echo "6. Duplicate Detection"
echo "   Attempting to add duplicate..."
test_api "Duplicate detection" "POST" "/add" \
    '{"text":"Goats need 15-20 sq ft of shelter space per animal.","title":"Duplicate Test"}' \
    '"success":false'

echo ""
echo "7. Legacy Endpoints"
echo "   Testing legacy GET /search..."
test_api "Legacy search" "GET" "/search?q=goat&k=3" "" '"success":true'

echo ""
echo "================================"
echo -e "${GREEN}Tests Passed: $TESTS_PASSED${NC}"
if [ $TESTS_FAILED -gt 0 ]; then
    echo -e "${RED}Tests Failed: $TESTS_FAILED${NC}"
    exit 1
else
    echo -e "${GREEN}All tests passed! ✓${NC}"
    echo ""
    echo "🚀 Brain Server v6.2 is ready!"
    echo ""
    echo "📖 Quick Test Commands:"
    echo "   # Add knowledge"
    echo '   curl -X POST http://127.0.0.1:8765/add \'
    echo '     -H "Content-Type: application/json" \'
    echo '     -d '"'"'{"text":"Your text here","title":"Title"}'"'"
    echo ""
    echo "   # Search"
    echo '   curl -X POST http://127.0.0.1:8765/search \'
    echo '     -H "Content-Type: application/json" \'
    echo '     -d '"'"'{"query":"Your search query","k":10}'"'"
    echo ""
fi
